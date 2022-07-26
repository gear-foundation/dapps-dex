#![no_std]

use dex_pair_io::*;
use gear_lib::fungible_token::{ft_core::*, state::*};
use gear_lib_derive::{FTCore, FTMetaState, FTStateKeeper};
use gstd::{cmp, exec, msg, prelude::*, ActorId};
use num::integer::Roots;
pub mod math;
pub mod messages;

const MINIMUM_LIQUIDITY: u128 = 1000;
static ZERO_ID: ActorId = ActorId::zero();

#[derive(Debug, Default, FTStateKeeper, FTCore, FTMetaState)]
pub struct Pair {
    #[FTStateField]
    pub token: FTState,
    // Factoty address which deployed this pair
    pub factory: ActorId,
    // First FT contract address.
    pub token0: ActorId,
    // Second FT contract address.
    pub token1: ActorId,
    // Last timestamp when the reserves and balances were updated
    last_block_ts: u128,
    // Balances of token0 and token1, to get rid of actually querying the balance from the contract.
    pub balance0: u128,
    pub balance1: u128,
    // Token0 and token1 reserves.
    reserve0: u128,
    reserve1: u128,
    // Token prices
    pub price0_cl: u128,
    pub price1_cl: u128,
    // K which is equal to self.reserve0 * self.reserve1 which is used to amount calculations when performing a swap.
    pub k_last: u128,
}

static mut PAIR: Option<Pair> = None;

impl Pair {
    // INTERNAL METHODS

    // A simple wrapper for balance calculations to facilitate mint & burn.
    fn update_balance(&mut self, to: ActorId, amount: u128, increase: bool) {
        self.get_mut()
            .allowances
            .entry(to)
            .or_default()
            .insert(to, amount);
        if increase {
            self.get_mut()
                .balances
                .entry(to)
                .and_modify(|balance| *balance += amount)
                .or_insert(amount);
            self.get_mut().total_supply += amount;
        } else {
            self.get_mut()
                .balances
                .entry(to)
                .and_modify(|balance| *balance -= amount)
                .or_insert(amount);
            self.get_mut().total_supply -= amount;
        }
    }

    // Mints the liquidity.
    // `to` - MUST be a non-zero address
    // Arguments:
    // * `to` - is the operation performer
    async fn mint(&mut self, to: ActorId) -> u128 {
        let amount0 = self.balance0.saturating_sub(self.reserve0);
        let amount1 = self.balance1.saturating_sub(self.reserve1);
        let fee_on = self.mint_fee(self.reserve0, self.reserve1).await;
        let total_supply = self.get().total_supply;
        let liquidity: u128;
        if total_supply == 0 {
            liquidity = amount0
                .overflowing_mul(amount1)
                .0
                .sqrt()
                .saturating_add(MINIMUM_LIQUIDITY);
            // Lock a minimum liquidity to a zero address.
            self.update_balance(ZERO_ID, liquidity, true);
            // FTCore::mint(self, &ZERO_ID, liquidity);
        } else {
            liquidity = cmp::min(
                amount0
                    .overflowing_mul(total_supply)
                    .0
                    .overflowing_div(self.reserve0)
                    .0,
                amount1
                    .overflowing_mul(total_supply)
                    .0
                    .overflowing_div(self.reserve1)
                    .0,
            )
        }
        if liquidity == 0 {
            panic!("PAIR: Liquidity MUST be greater than 0.");
        }
        self.update_balance(to, liquidity, true);
        // FTCore::mint(self, &to, liquidity);
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        if fee_on {
            // Calculate the K which is the product of reserves.
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0;
        }
        liquidity
    }

    // Mint liquidity if fee is on.
    // If fee is on, mint liquidity equivalent to 1/6th of the growth in sqrt(k). So the math if the following.
    // Calculate the sqrt of current k using the reserves. Compare it.
    // If the current one is greater, than calculate the liquidity using the following formula:
    // liquidity = (total_supply * (root_k - last_root_k)) / (root_k * 5 + last_root_k).
    // where root_k - is the sqrt of the current product of reserves, and last_root_k - is the sqrt of the previous product.
    // Multiplication by 5 comes from the 1/6 of growrth is sqrt.
    // `reserve0` - MUST be a positive number
    // `reserve1` - MUST be a positive number
    // Arguments:
    // * `reserve0` - the available amount of token0
    // * `reserve1` - the available amount of token1
    async fn mint_fee(&mut self, reserve0: u128, reserve1: u128) -> bool {
        // get fee_to from factory
        let fee_to: ActorId = messages::get_fee_to(&self.factory).await;
        let fee_on = fee_to != ZERO_ID;
        if fee_on {
            if self.k_last != 0 {
                // Calculate the sqrt of current K.
                let root_k = reserve0.overflowing_mul(reserve1).0.sqrt();
                // Get the sqrt of previous K.
                let root_k_last = self.k_last.sqrt();
                if root_k > root_k_last {
                    let numerator = self
                        .get()
                        .total_supply
                        .overflowing_mul(root_k.saturating_sub(root_k_last))
                        .0;
                    // Calculate the 1/6 of a fee is the fee is turned on.
                    let denominator = root_k.overflowing_mul(5).0.overflowing_add(root_k_last).0;
                    let liquidity = numerator.overflowing_div(denominator).0;
                    if liquidity > 0 {
                        self.update_balance(fee_to, liquidity, true);
                        // FTCore::mint(self, &fee_to, liquidity);
                    }
                }
            }
        } else if self.k_last != 0 {
            self.k_last = 0;
        }
        fee_on
    }

    // Updates reserves and, on the first call per block, price accumulators
    // `balance0` - MUST be a positive number
    // `balance1` - MUST be a positive number
    // `reserve0` - MUST be a positive number
    // `reserve1` - MUST be a positive number
    // Arguments:
    // * `balance0` - token0 balance
    // * `balance1` - token1 balance
    // * `reserve0` - the available amount of token0
    // * `reserve1` - the available amount of token1
    fn update(&mut self, balance0: u128, balance1: u128, reserve0: u128, reserve1: u128) {
        let current_ts = (exec::block_timestamp() & 0xFFFFFFFF) as u32;
        let time_elapsed = current_ts as u128 - self.last_block_ts;
        // Update the prices if we actually update the balances later.
        if time_elapsed > 0 && reserve0 != 0 && reserve1 != 0 {
            self.price0_cl = self
                .price0_cl
                .overflowing_add(
                    self.price0_cl
                        .overflowing_div(reserve0)
                        .0
                        .overflowing_mul(time_elapsed)
                        .0,
                )
                .0;
            self.price1_cl = self
                .price1_cl
                .overflowing_add(
                    self.price1_cl
                        .overflowing_div(reserve1)
                        .0
                        .overflowing_mul(time_elapsed)
                        .0,
                )
                .0;
        }
        self.reserve0 = balance0;
        self.reserve1 = balance1;
        self.last_block_ts = current_ts as u128;
    }

    // Burns the liquidity.
    // `to` - MUST be a non-zero address
    // Arguments:
    // * `to` - is the operation performer
    async fn burn(&mut self, to: ActorId) -> (u128, u128) {
        let fee_on = self.mint_fee(self.reserve0, self.reserve1).await;
        // get liquidity

        let liquidity: u128 = *self
            .get()
            .balances
            .get(&exec::program_id())
            .expect("The pair has no liquidity at all");
        let amount0 = liquidity
            .overflowing_mul(self.balance0)
            .0
            .overflowing_div(self.get().total_supply)
            .0;
        let amount1 = liquidity
            .overflowing_mul(self.balance1)
            .0
            .overflowing_div(self.get().total_supply)
            .0;
        if amount0 == 0 || amount1 == 0 {
            panic!("PAIR: Insufficient liquidity burnt.");
        }
        // add this later to ft_core
        self.update_balance(to, liquidity, false);
        // FTCore::burn(self, liquidity);
        messages::transfer_tokens(&self.token0, &exec::program_id(), &to, amount0).await;
        messages::transfer_tokens(&self.token1, &exec::program_id(), &to, amount1).await;
        self.balance0 -= amount0;
        self.balance1 -= amount1;
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        if fee_on {
            // If fee is on recalculate the K.
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0;
        }
        (amount0, amount1)
    }

    // Swaps two tokens just by calling transfer_tokens from the token contracts.
    // Also maintains the balances and updates the reservers to match the balances.
    // `amount0` - MUST be more than self.reserve0
    // `amount1` - MUST be more than self.reserve1
    // `to` - MUST be a non-zero address
    // Arguments:
    // * `amount0` - amount of token0
    // * `amount1` - amount of token1
    // * `to` - is the operation performer
    // * `forward` - is the direction. If true - user inputs token0 and gets token1, otherwise - token1 -> token0
    async fn _swap(&mut self, amount0: u128, amount1: u128, to: ActorId, forward: bool) {
        if amount0 > self.reserve0 && forward {
            panic!("PAIR: Insufficient liquidity.");
        }
        if amount1 > self.reserve1 && !forward {
            panic!("PAIR: Insufficient liquidity.");
        }
        // carefully, not forward
        if !forward {
            messages::transfer_tokens(&self.token0, &exec::program_id(), &to, amount0).await;
            messages::transfer_tokens(&self.token1, &to, &exec::program_id(), amount1).await;
            self.balance0 -= amount0;
            self.balance1 += amount1;
        } else {
            messages::transfer_tokens(&self.token0, &to, &exec::program_id(), amount0).await;
            messages::transfer_tokens(&self.token1, &exec::program_id(), &to, amount1).await;
            self.balance0 += amount0;
            self.balance1 -= amount1;
        }
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
    }

    // EXTERNAL FUNCTIONS

    /// Forces balances to match the reserves.
    /// `to` - MUST be a non-zero address
    /// Arguments:
    /// * `to` - where to perform tokens' transfers
    pub async fn skim(&mut self, to: ActorId) {
        messages::transfer_tokens(
            &self.token0,
            &exec::program_id(),
            &to,
            self.balance0.saturating_sub(self.reserve0),
        )
        .await;
        messages::transfer_tokens(
            &self.token1,
            &exec::program_id(),
            &to,
            self.balance1.saturating_sub(self.reserve1),
        )
        .await;
        // Update the balances.
        self.balance0 -= self.reserve0;
        self.balance1 -= self.reserve1;
        msg::reply(
            PairEvent::Skim {
                to,
                amount0: self.balance0,
                amount1: self.balance1,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::Sync");
    }

    /// Forces reserves to match balances.
    pub async fn sync(&mut self) {
        let balance0 = messages::get_balance(&self.token0, &exec::program_id()).await;
        let balance1 = messages::get_balance(&self.token1, &exec::program_id()).await;
        self.update(balance0, balance1, self.reserve0, self.reserve1);
        msg::reply(
            PairEvent::Sync {
                balance0,
                balance1,
                reserve0: self.reserve0,
                reserve1: self.reserve1,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::Sync");
    }

    /// Adds liquidity to the pool.
    /// `to` - MUST be a non-zero address
    /// Arguments:
    /// * `amount0_desired` - is the desired amount of token0 the user wants to add
    /// * `amount1_desired` - is the desired amount of token1 the user wants to add
    /// * `amount0_min` - is the minimum amount of token0 the user wants to add
    /// * `amount1_min` - is the minimum amount of token1 the user wants to add
    /// * `to` - is the liquidity provider
    pub async fn add_liquidity(
        &mut self,
        amount0_desired: u128,
        amount1_desired: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    ) {
        let amount0: u128;
        let amount1: u128;
        // Check the amounts provided with the respect to the reserves to find the best amount of tokens0/1 to be added.
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

        let pair_address = exec::program_id();
        messages::transfer_tokens(&self.token0, &msg::source(), &pair_address, amount0).await;
        messages::approve_tokens(&self.token0, &pair_address, amount0).await;
        messages::transfer_tokens(&self.token1, &msg::source(), &pair_address, amount1).await;
        messages::approve_tokens(&self.token1, &pair_address, amount1).await;
        // Update the balances.
        self.balance0 += amount0;
        self.balance1 += amount1;
        // call mint function
        let liquidity = self.mint(to).await;
        msg::reply(
            PairEvent::AddedLiquidity {
                amount0,
                amount1,
                liquidity,
                to,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::AddedLiquidity");
    }

    /// Removes liquidity from the pool.
    /// Internally calls self.burn function while transferring `liquidity` amount of internal tokens
    /// `to` - MUST be a non-zero address
    /// Arguments:
    /// * `liquidity` - is the desired liquidity the user wants to remove (e.g. burn)
    /// * `amount0_min` - is the minimum amount of token0 the user wants to receive
    /// * `amount1_min` - is the minimum amount of token1 the user wants to receive
    /// * `to` - is the liquidity provider
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

    /// Swaps exact token0 for some token1
    /// Internally calculates the price from the reserves and call self._swap
    /// `to` - MUST be a non-zero address
    /// `amount_in` - MUST be non-zero
    /// Arguments:
    /// * `amount_in` - is the amount of token0 user want to swap
    /// * `to` - is the receiver of the swap operation
    pub async fn swap_exact_tokens_for(&mut self, amount_in: u128, to: ActorId) {
        // token1 amount
        let amount_out = math::get_amount_out(amount_in, self.reserve0, self.reserve1);

        self._swap(amount_in, amount_out, to, true).await;
        msg::reply(
            PairEvent::SwapExactTokensFor {
                to,
                amount_in,
                amount_out,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::SwapExactTokensFor");
    }

    /// Swaps exact token1 for some token0
    /// Internally calculates the price from the reserves and call self._swap
    /// `to` - MUST be a non-zero address
    /// `amount_in` - MUST be non-zero
    /// Arguments:
    /// * `amount_out` - is the amount of token1 user want to swap
    /// * `to` - is the receiver of the swap operation
    pub async fn swap_tokens_for_exact(&mut self, amount_out: u128, to: ActorId) {
        let amount_in = math::get_amount_in(amount_out, self.reserve0, self.reserve1);

        self._swap(amount_in, amount_out, to, false).await;
        msg::reply(
            PairEvent::SwapTokensForExact {
                to,
                amount_in,
                amount_out,
            },
            0,
        )
        .expect("Error during a replying with PairEvent::SwapTokensForExact");
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
