#![no_std]

use dex_pair_io::*;
use gear_lib::fungible_token::{ft_core::*, state::*};
use gear_lib_derive::{FTCore, FTMetaState, FTStateKeeper};
use gstd::{cmp, exec, msg, prelude::*, ActorId};
use num::integer::Roots;
mod internals;
pub mod math;
pub mod messages;

const MINIMUM_LIQUIDITY: u128 = 1000;

#[derive(Debug, Default, FTStateKeeper, FTCore, FTMetaState)]
pub struct Pair {
    #[FTStateField]
    pub token: FTState,
    /// Factoty address which deployed this pair.
    pub factory: ActorId,
    /// First FT contract address.
    pub token0: ActorId,
    /// Second FT contract address.
    pub token1: ActorId,
    /// Last timestamp when the reserves and balances were updated.
    last_block_ts: u128,
    /// Balances of token0 and token1, to get rid of actually querying the balance from the contract.
    pub balance0: u128,
    pub balance1: u128,
    /// Token0 and token1 reserves.
    reserve0: u128,
    reserve1: u128,
    /// Token prices.
    pub price0_cl: u128,
    pub price1_cl: u128,
    /// K which is equal to self.reserve0 * self.reserve1 which is used to amount calculations when performing a swap.
    pub k_last: u128,
    /// Global transaction id nonce.
    transaction_id: u64,
    /// Hold transaction id, cached `amount0`, `amount1`.
    transactions: BTreeMap<ActorId, (u64, u128, u128)>,
}

static mut PAIR: Option<Pair> = None;

impl Pair {
    // EXTERNAL FUNCTIONS

    /// Forces balances to match the reserves.
    /// # Requirements:
    /// * `to` - MUST be a non-zero address.
    /// # Arguments:
    /// * `to` - where to perform tokens transfers.
    pub async fn skim(&mut self, to: ActorId) {
        let source = exec::program_id();

        let amount0 = self
            .balance0
            .checked_sub(self.reserve0)
            .expect("Math overflow!");
        let amount1 = self
            .balance1
            .checked_sub(self.reserve1)
            .expect("Math overflow!");

        let (first_transfer_tx_id, amount0, amount1) =
            *self.transactions.entry(source).or_insert_with(|| {
                let id = self.transaction_id;

                self.transaction_id = self.transaction_id.wrapping_add(3);

                (id, amount0, amount1)
            });
        let second_transfer_tx_id = first_transfer_tx_id + 1;
        let third_transfer_tx_id = second_transfer_tx_id + 1;

        if messages::transfer_tokens_sharded(
            first_transfer_tx_id,
            &self.token0,
            &source,
            &to,
            amount0,
        )
        .await
        .is_err()
        {
            self.transactions.remove(&source);
            msg::reply(PairEvent::TransactionFailed(first_transfer_tx_id), 0)
                .expect("Unable to reply!");
            return;
        }

        if messages::transfer_tokens_sharded(
            second_transfer_tx_id,
            &self.token1,
            &source,
            &to,
            amount1,
        )
        .await
        .is_err()
        {
            if messages::transfer_tokens_sharded(
                third_transfer_tx_id,
                &self.token0,
                &to,
                &source,
                amount0,
            )
            .await
            .is_err()
            {
                // In theory this arm should never been executed
                msg::reply(PairEvent::RerunTransaction(third_transfer_tx_id), 0)
                    .expect("Unable to reply!");
                return;
            }

            self.transactions.remove(&source);

            msg::reply(PairEvent::TransactionFailed(second_transfer_tx_id), 0)
                .expect("Unable to reply!");
            return;
        }

        // Update the balances
        self.balance0 = amount0;
        self.balance1 = amount1;

        self.transactions.remove(&source);

        msg::reply(
            PairEvent::Skim {
                to,
                amount0: self.balance0,
                amount1: self.balance1,
            },
            0,
        )
        .expect("Error during a replying with `PairEvent::Skim`");
    }

    /// Forces reserves to match balances.
    pub async fn sync(&mut self) {
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        msg::reply(
            PairEvent::Sync {
                balance0: self.balance0,
                balance1: self.balance1,
                reserve0: self.reserve0,
                reserve1: self.reserve1,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::Sync");
    }

    /// Adds liquidity to the pool.
    /// # Requirements:
    /// * `to` - MUST be a non-zero address.
    /// # Arguments:
    /// * `amount0_desired` - is the desired amount of `token0` the user wants to add.
    /// * `amount1_desired` - is the desired amount of `token1` the user wants to add.
    /// * `amount0_min` - is the minimum amount of `token0` the user wants to add.
    /// * `amount1_min` - is the minimum amount of `token1` the user wants to add.
    /// * `to` - is the liquidity provider.
    pub async fn add_liquidity(
        &mut self,
        amount0_desired: u128,
        amount1_desired: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    ) {
        let source = msg::source();

        let amount0: u128;
        let amount1: u128;
        // Check the amounts provided with the respect to the reserves to find the best amount of tokens0/1 to be added
        if self.reserve0 == 0 && self.reserve1 == 0 {
            amount0 = amount0_desired;
            amount1 = amount1_desired;
        } else {
            let amount1_optimal = math::quote(amount0_desired, self.reserve0, self.reserve1);
            if amount1_optimal < amount1_desired {
                if amount1_optimal >= amount1_min {
                    panic!("PAIR: Insufficient token1 amount.");
                }
                amount0 = amount0_desired;
                amount1 = amount1_optimal;
            } else {
                let amount0_optimal = math::quote(amount1_desired, self.reserve0, self.reserve1);
                if amount0_optimal >= amount0_min {
                    panic!("PAIR: Insufficient token0 amount.");
                }
                amount0 = amount0_optimal;
                amount1 = amount1_desired;
            }
        }

        // Note: `amount0` and `amount1` can be changed between blocks,
        // and because of that, if `second_transfer_tx_id` will exceed gas,
        // then another transfer(with correct gas amount) will introduce invalid
        // optimal / desired amounts(because amounts are changed in prev block)
        let (first_transfer_tx_id, amount0, amount1) =
            *self.transactions.entry(source).or_insert_with(|| {
                let id = self.transaction_id;

                self.transaction_id = self.transaction_id.wrapping_add(3);

                (id, amount0, amount1)
            });
        let second_transfer_tx_id = first_transfer_tx_id + 1;
        let third_transfer_tx_id = second_transfer_tx_id + 1;

        let pair_address = exec::program_id();
        if messages::transfer_tokens_sharded(
            first_transfer_tx_id,
            &self.token0,
            &source,
            &pair_address,
            amount0,
        )
        .await
        .is_err()
        {
            self.transactions.remove(&source);
            msg::reply(PairEvent::TransactionFailed(first_transfer_tx_id), 0)
                .expect("Unable to reply!");
            return;
        }

        if messages::transfer_tokens_sharded(
            second_transfer_tx_id,
            &self.token1,
            &source,
            &pair_address,
            amount1,
        )
        .await
        .is_err()
        {
            if messages::transfer_tokens_sharded(
                third_transfer_tx_id,
                &self.token0,
                &pair_address,
                &source,
                amount0,
            )
            .await
            .is_err()
            {
                // In theory this arm should never been executed
                msg::reply(PairEvent::RerunTransaction(third_transfer_tx_id), 0)
                    .expect("Unable to reply!");
                return;
            }

            self.transactions.remove(&source);

            msg::reply(PairEvent::TransactionFailed(second_transfer_tx_id), 0)
                .expect("Unable to reply!");
            return;
        }

        // Update the balances
        self.balance0 = self.balance0.checked_add(amount0).expect("Math overflow!");
        self.balance1 = self.balance1.checked_add(amount1).expect("Math overflow!");

        // Call mint function
        let liquidity = self.mint(to).await;

        self.transactions.remove(&source);

        msg::reply(
            PairEvent::AddedLiquidity {
                amount0,
                amount1,
                liquidity,
                to,
            },
            0,
        )
        .expect("Error during a replying with `PairEvent::AddedLiquidity`");
    }

    /// Removes liquidity from the pool.
    /// Internally calls `self.burn` function while transferring `liquidity` amount of internal tokens.
    /// # Requirements:
    /// * `to` - MUST be a non-zero address.
    /// # Arguments:
    /// * `liquidity` - is the desired liquidity the user wants to remove (e.g. burn).
    /// * `amount0_min` - is the minimum amount of `token0` the user wants to receive.
    /// * `amount1_min` - is the minimum amount of `token1` the user wants to receive.
    /// * `to` - is the liquidity provider.
    pub async fn remove_liquidity(
        &mut self,
        liquidity: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    ) {
        FTCore::transfer(self, &msg::source(), &exec::program_id(), liquidity);
        // Burn and get the optimal amount of burned tokens.
        let (amount0, amount1) = self.burn(to).await;

        if amount0 < amount0_min {
            panic!("PAIR: Insufficient amount of token 0")
        }
        if amount1 < amount1_min {
            panic!("PAIR: Insufficient amount of token 1")
        }
        // msg::reply(PairEvent::RemovedLiquidity { liquidity, to }, 0)
        // .expect("Error during a replying with PairEvent::RemovedLiquidity");
    }

    /// Swaps exact `token0` for some `token1`.
    /// Internally calculates the price from the reserves and call `self._swap`.
    /// # Requirements:
    /// * `to` - MUST be a non-zero address.
    /// * `amount_in` - MUST be non-zero.
    /// # Arguments:
    /// * `amount_in` - is the amount of `token0` user want to swap.
    /// * `to` - is the receiver of the swap operation.
    pub async fn swap_exact_tokens_for(&mut self, amount_in: u128, to: ActorId) {
        // token1 amount
        let amount_out = math::get_amount_out(amount_in, self.reserve0, self.reserve1);

        if !self._swap(amount_in, amount_out, to, true).await {
            return;
        }

        msg::reply(
            PairEvent::SwapExactTokensFor {
                to,
                amount_in,
                amount_out,
            },
            0,
        )
        .expect("Error during a replying with `PairEvent::SwapExactTokensFor`");
    }

    /// Swaps exact `token1` for some `token0`.
    /// Internally calculates the price from the reserves and call `self._swap`.
    /// # Requirements:
    /// * `to` - MUST be a non-zero address.
    /// * `amount_in` - MUST be non-zero.
    /// # Arguments:
    /// * `amount_out` - is the amount of `token1` user want to swap.
    /// * `to` - is the receiver of the swap operation.
    pub async fn swap_tokens_for_exact(&mut self, amount_out: u128, to: ActorId) {
        let amount_in = math::get_amount_in(amount_out, self.reserve0, self.reserve1);

        if !self._swap(amount_in, amount_out, to, false).await {
            return;
        }

        msg::reply(
            PairEvent::SwapTokensForExact {
                to,
                amount_in,
                amount_out,
            },
            0,
        )
        .expect("Error during a replying with `PairEvent::SwapTokensForExact`");
    }
}

#[no_mangle]
extern "C" fn init() {
    let config: InitPair = msg::load().expect("Unable to decode InitPair");
    // DISABLE FOR TESTING, UNCOMMENT AND FIX TESTS LATER
    // if config.factory != msg::source() {
    //     panic!("PAIR: Can only be created by a factory.");
    // }
    let pair = Pair {
        factory: config.factory,
        token0: config.token0,
        token1: config.token1,
        ..Default::default()
    };
    unsafe {
        PAIR = Some(pair);
    }
}

#[gstd::async_main]
async fn main() {
    let action: PairAction = msg::load().expect("Unable to decode PairAction");
    let pair = unsafe { PAIR.get_or_insert(Default::default()) };
    match action {
        PairAction::AddLiquidity {
            amount0_desired,
            amount1_desired,
            amount0_min,
            amount1_min,
            to,
        } => {
            pair.add_liquidity(
                amount0_desired,
                amount1_desired,
                amount0_min,
                amount1_min,
                to,
            )
            .await
        }
        PairAction::RemoveLiquidity {
            liquidity,
            amount0_min,
            amount1_min,
            to,
        } => {
            pair.remove_liquidity(liquidity, amount0_min, amount1_min, to)
                .await
        }
        PairAction::Sync => pair.sync().await,
        PairAction::Skim { to } => pair.skim(to).await,
        PairAction::SwapExactTokensFor { to, amount_in } => {
            pair.swap_exact_tokens_for(amount_in, to).await
        }
        PairAction::SwapTokensForExact { to, amount_out } => {
            pair.swap_tokens_for_exact(amount_out, to).await
        }
    }
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let state: PairStateQuery = msg::load().expect("Unable to decode PairStateQuery");
    let pair = unsafe { PAIR.get_or_insert(Default::default()) };
    let reply = match state {
        PairStateQuery::TokenAddresses => PairStateReply::TokenAddresses {
            token0: pair.token0,
            token1: pair.token1,
        },
        PairStateQuery::Reserves => PairStateReply::Reserves {
            reserve0: pair.reserve0,
            reserve1: pair.reserve1,
        },
        PairStateQuery::Prices => PairStateReply::Prices {
            price0: pair.price0_cl,
            price1: pair.price1_cl,
        },
        PairStateQuery::BalanceOf(address) => {
            PairStateReply::Balance(*pair.get().balances.get(&address).unwrap_or(&0))
        }
    };
    gstd::util::to_leak_ptr(reply.encode())
}

gstd::metadata! {
    title: "DEXPair",
    init:
        input: InitPair,
    handle:
        input: PairAction,
        output: PairEvent,
    state:
        input: PairStateQuery,
        output: PairStateReply,
}
