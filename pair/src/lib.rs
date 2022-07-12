#![no_std]

use gear_lib::fungible_token::{ft_core::*, state::*};
use gear_lib_derive::{FTCore, FTMetaState, FTStateKeeper};
use gstd::{cmp, exec, msg, prelude::*, ActorId};
use num::integer::Roots;
use pair_io::*;
pub mod math;
pub mod messages;

const MINIMUM_LIQUIDITY: u128 = 1000;
static ZERO_ID: ActorId = ActorId::zero();

#[derive(Debug, Default, FTStateKeeper, FTCore, FTMetaState)]
pub struct Pair {
    #[FTStateField]
    pub token: FTState,
    pub factory: ActorId,
    pub token0: ActorId,
    pub token1: ActorId,
    last_block_ts: u128,
    pub balance0: u128,
    pub balance1: u128,
    reserve0: u128,
    reserve1: u128,
    pub price0_cl: u128,
    pub price1_cl: u128,
    pub k_last: u128,
}

static mut PAIR: Option<Pair> = None;

impl Pair {
    // INTERNAL METHODS
    fn _mint(&mut self, to: ActorId) -> u128 {
        let amount0 = self.balance0.overflowing_sub(self.reserve0).0;
        let amount1 = self.balance1.overflowing_sub(self.reserve1).0;
        let fee_on = self._mint_fee(self.reserve0, self.reserve1);
        let total_supply = self.get().total_supply;
        let liquidity: u128;
        if total_supply == 0 {
            // Math.sqrt(amount0.mul(amount1)).sub(MINIMUM_LIQUIDITY);
            liquidity = amount0
                .overflowing_mul(amount1)
                .0
                .sqrt()
                .overflowing_sub(MINIMUM_LIQUIDITY)
                .0;
            // add this later to ft lib
            FTCore::mint(self, &ZERO_ID, liquidity);
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
        FTCore::mint(self, &to, liquidity);
        self._update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        if fee_on {
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0;
        }
        liquidity
    }

    fn _mint_fee(&mut self, reserve0: u128, reserve1: u128) -> bool {
        // get fee_to from factory
        let fee_to: ActorId = ActorId::zero();
        let fee_on = fee_to != ZERO_ID;
        if fee_on {
            if self.k_last != 0 {
                let root_k = reserve0.overflowing_mul(reserve1).0;
                let root_k_last = self.k_last.sqrt();
                if root_k > root_k_last {
                    let numerator = self
                        .get()
                        .total_supply
                        .overflowing_mul(root_k.overflowing_sub(root_k_last).0)
                        .0;
                    let denominator = root_k.overflowing_mul(5).0.overflowing_add(root_k_last).0;
                    let liquidity = numerator.overflowing_div(denominator).0;
                    if liquidity > 0 {
                        FTCore::mint(self, &fee_to, liquidity);
                    }
                }
            }
        } else if self.k_last != 0 {
            self.k_last = 0;
        }
        fee_on
    }

    fn _update(&mut self, balance0: u128, balance1: u128, reserve0: u128, reserve1: u128) {
        let current_ts = exec::block_timestamp() % (1 << 32);
        let time_elapsed = current_ts as u128 - self.last_block_ts;
        if time_elapsed > 0 && reserve0 != 0 && reserve1 != 0 {
            self.price0_cl = self.price0_cl.overflowing_add(
                self.price0_cl
                    .overflowing_div(reserve0)
                    .0
                    .overflowing_mul(time_elapsed)
                    .0,
            ).0;
            self.price1_cl = self.price1_cl.overflowing_add(
                self.price1_cl
                    .overflowing_div(reserve1)
                    .0
                    .overflowing_mul(time_elapsed)
                    .0,
            ).0;
        }
        self.reserve0 = balance0;
        self.reserve1 = balance1;
        self.last_block_ts = current_ts as u128;
    }

    async fn _burn(&mut self, to: ActorId) -> (u128, u128){
        let fee_on = self._mint_fee(self.reserve0, self.reserve1);
        // get liquidity
        let liquidity: u128 = 0;
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
        FTCore::burn(self, liquidity);

        // do not get what _safetransfer is
        // _safeTransfer(_token0, to, amount0);
        messages::transfer_tokens(&self.token0, &exec::program_id(), &to, amount0).await;
        // _safeTransfer(_token1, to, amount1);
        messages::transfer_tokens(&self.token1, &exec::program_id(), &to, amount1).await;
        self.balance0 -= amount0;
        self.balance1 -= amount1;
        self._update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        if fee_on {
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0;
        }
        (amount0, amount1)
    }

    async fn _swap(&mut self, amount0_out: u128, amount1_out: u128, to: ActorId) {
        if amount0_out > self.reserve0 || amount1_out > self.reserve1 {
            panic!("PAIR: Insufficient liquidity.");
        }
        if to == self.token0 || to == self.token1 {
            panic!("PAIR: to MUST be different from token pools addresses.");
        }
        if amount0_out > 0 {
            messages::transfer_tokens(&self.token0, &exec::program_id(), &to, amount0_out).await;
        }
        if amount1_out > 0 {
            messages::transfer_tokens(&self.token1, &exec::program_id(), &to, amount1_out).await;
        }
        self.balance0 -= amount0_out;
        self.balance1 -= amount1_out;
        // let amount0_in = if self.balance0 > self.reserve0 - amount0_out {
        //     self.balance0 - (self.reserve0 - amount0_out)
        // } else {
        //     0
        // };
        // let amount1_in = if self.balance1 > self.reserve1 - amount0_out {
        //     self.balance1 - (self.reserve1 - amount0_out)
        // } else {
        //     0
        // };
        self._update(self.balance0, self.balance1, self.reserve0, self.reserve1);
    }

    // EXTERNAL STUFF
    pub async fn skim(&mut self, to: ActorId) {
        messages::transfer_tokens(
            &self.token0,
            &exec::program_id(),
            &to,
            self.balance0.overflowing_sub(self.reserve0).0,
        )
        .await;
        messages::transfer_tokens(
            &self.token1,
            &exec::program_id(),
            &to,
            self.balance1.overflowing_sub(self.reserve1).0,
        )
        .await;
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

    pub async fn sync(&mut self) {
        let balance0 = messages::get_balance(&self.token0, &exec::program_id()).await;
        let balance1 = messages::get_balance(&self.token1, &exec::program_id()).await;
        self._update(balance0, balance1, self.reserve0, self.reserve1);
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
        messages::transfer_tokens(&self.token1, &msg::source(), &pair_address, amount1).await;
        self.balance0 += amount0;
        self.balance1 += amount1;

        // call mint function
        let liquidity = self._mint(to);
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

    pub async fn remove_liquidity(
        &mut self,
        liquidity: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    ) {
        FTCore::transfer(self, &msg::source(), &exec::program_id(), liquidity);
        // call burn
        let (amount0, amount1) = self._burn(to).await;
        if amount0 > amount0_min {
            panic!("PAIR: Insufficient amount of token 0")
        }
        if amount1 > amount1_min {
            panic!("PAIR: Insufficient amount of token 1")
        }
        msg::reply(PairEvent::RemovedLiquidity { liquidity, to }, 0)
            .expect("Error during a replying with PairEvent::RemovedLiquidity");
    }

    pub async fn swap_exact_tokens_for(&mut self, amount_in: u128, to: ActorId) {
        let amount_out = math::get_amount_out(amount_in, self.reserve0, self.reserve1);
        messages::transfer_tokens(&self.token0, &exec::program_id(), &to, amount_out).await;
        self._swap(0, amount_out, to).await;
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

    pub async fn swap_tokens_for_exact(&mut self, amount_out: u128, to: ActorId) {
        let amount_in = math::get_amount_in(amount_out, self.reserve0, self.reserve1);
        messages::transfer_tokens(&self.token1, &exec::program_id(), &to, amount_out).await;
        self._swap(amount_in, 0, to).await;
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
    let config: InitPair = msg::load().expect("Unable to decode InitEscrow");
    if config.factory != msg::source() {
        panic!("PAIR: Can only be created by a factory.");
    }
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
async unsafe fn main() {
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
        PairAction::Sync {} => pair.sync().await,
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
        PairStateQuery::TokenAddresses {} => PairStateReply::TokenAddresses {
            token0: pair.token0,
            token1: pair.token1,
        },
        PairStateQuery::Reserves {} => PairStateReply::Reserves {
            reserve0: pair.reserve0,
            reserve1: pair.reserve1,
        },
        PairStateQuery::Prices {} => PairStateReply::Prices {
            price0: pair.price0_cl,
            price1: pair.price1_cl,
        },
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
