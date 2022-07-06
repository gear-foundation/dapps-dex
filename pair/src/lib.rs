// #![no_std]

use pair_io::*;
use gstd::{msg, prelude::*, ActorId};
use gear_lib_derive::{FTCore, FTMetaState, FTStateKeeper};
use num::integer::Roots;
use std::cmp;
pub mod utils;
use utils::*;

const MINIMUM_LIQUIDITY: u128 = 1000;
static ZERO_ID: ActorId = ActorId::zero();

#[derive(Debug, Default)]
pub struct Pair {
    #[FTStateField]
    pub factory: ActorId,
    pub token_0: ActorId,
    pub token_1: ActorId,
    last_block_ts: u128,
    reserve0: u128,
    reserve1: u128,
    pub price_0_cl: u128,
    pub price_1_cl: u128,
    pub k_last: u128,
}

static mut PAIR: Option<Pair> = None;

impl Pair {
    pub fn mint(&mut self, to: ActorId) {
        // get balance0
        let balance0: u128 = 10;
        // get balance1
        let balance1: u128 = 10;
        let amount0 = balance0.overflowing_sub(self.reserve0).0;
        let amount1 = balance1.overflowing_sub(self.reserve1).0;

        let feeOn = self._mint_fee(self.reserve0, self.reserve1);
        // get total_supply and perform math
        let total_supply = self.get().total_supply;
        let liquidity: u128;
        if total_supply == 0 {
            liquidity = amount0.overflowing_mul(amount1).0.overflowing_sub(MINIMUM_LIQUIDITY).0.sqrt();
            // _mint(address(0), MINIMUM_LIQUIDITY); // permanently lock the first MINIMUM_LIQUIDITY tokens
        } else {
            liquidity = cmp::min(
                amount0.overflowing_mul(total_supply).0.overflowing_div(self.reserve0).0,
                amount1.overflowing_mul(total_supply).0.overflowing_div(self.reserve1).0,
            );
        }
        if liquidity <= 0 {
            panic!("PAIR: Liquidity should be >0");
        }

        // _mint(to, liquidity);
        self._update(balance0, balance1, self.reserve0, self.reserve1);

        if feeOn {
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0; // reserve0 and reserve1 are up-to-date
        }
    }

    pub fn burn(&mut self, to: ActorId) {
        // get balance0
        let balance0: u128 = 10;
        // get balance1
        let balance1: u128 = 10;

        // get liquidity (amount of tokens on this contract_address)
        let liquidity: u128 = 10;
        let feeOn = self._mint_fee(self.reserve0, self.reserve1);
        let total_supply = self.get().total_supply;
        let amount0 = liquidity.overflowing_mul(balance0).0.overflowing_div(total_supply).0;
        let amount1 = liquidity.overflowing_mul(balance1).0.overflowing_div(total_supply).0;
        if amount0 < 0 || amount1 < 0 {
            panic!("PAIR: Not enought liquidity burnt");
        }
        // _burn
        // _safeTransfer(_token0, to, amount0);
        // _safeTransfer(_token1, to, amount1);
        // get new balanes
        // balance0 = IERC20(_token0).balanceOf(address(this));
        // balance1 = IERC20(_token1).balanceOf(address(this));

        self._update(balance0, balance1, self.reserve0, self.reserve1);
        if feeOn {
            self.k_last = self.reserve0.overflowing_mul(self.reserve1).0; // reserve0 and reserve1 are up-to-date
        }
        msg::reply(
            PairEvent::Burn {
                sender: msg::source(),
                to,
                amount0,
                amount1,
            },
            0,
        )
        .expect("PAIR: Error during a replying with PairAction::Burn");
    }

    pub fn swap(&mut self, amount0_out: u128, amount1_out: u128, to: ActorId) {
        // do not get it why this is
        // require(amount0Out > 0 || amount1Out > 0, 'UniswapV2: INSUFFICIENT_OUTPUT_AMOUNT');
        if amount0_out < self.reserve0 && amount1_out < self.reserve1 {
            panic!("PAIR: Not enought liquidity.");
        }
        if self.token_0 == to || self.token_1 == to {
            panic!("PAIR: Can not send to one of the token addresses");
        }
        // if (amount0Out > 0) _safeTransfer(_token0, to, amount0Out); // optimistically transfer tokens
        // if (amount1Out > 0) _safeTransfer(_token1, to, amount1Out); // optimistically transfer tokens
        // get balances
        let balance0: u128 = 10;
        let balance1: u128 = 10;
        // balance0 > _reserve0 - amount0Out ? balance0 - (_reserve0 - amount0Out) : 0;
        let amount0_in: u128 = if balance0 > self.reserve0 - amount0_out { balance0 - (self.reserve0 - amount0_out)} else { 0 };
        let amount1_in: u128 = if balance1 > self.reserve1 - amount1_out { balance1 - (self.reserve1 - amount1_out)} else { 0 };
        // require(amount0In > 0 || amount1In > 0, 'UniswapV2: INSUFFICIENT_INPUT_AMOUNT');
        let balance0_adjusted = balance0.overflowing_mul(1000).0.overflowing_sub(
            amount0_in.overflowing_mul(3).0
        ).0;
        let balance1_adjusted = balance1.overflowing_mul(1000).0.overflowing_sub(
            amount1_in.overflowing_mul(3).0
        ).0;
        if balance0_adjusted.overflowing_mul(balance1_adjusted).0 > self.reserve0.overflowing_mul(self.reserve1).0.overflowing_mul(1000 * 1000).0 {
            panic!("PAIR: K violation.");
        }

        self._update(balance0, balance1, self.reserve0, self.reserve1);
        msg::reply(
            PairEvent::Swap {
                sender: msg::source(),
                amount0_in,
                amount0_out,
                amount1_in,
                amount1_out,
                to,
            },
            0,
        )
        .expect("PAIR: Error during a replying with PairAction::Swap");
    }

    pub fn skim(self, to: ActorId) {
        // get balance0
        // get balance1
        // safeTransfer token0
        // safeTransfer token1
        msg::reply(
            PairEvent::Skim {
                to,
            },
            0,
        )
        .expect("PAIR: Error during a replying with PairAction::Skim");
    }

    pub fn sync(self) {
        // get balance0
        // get balance1
        self._update(0, 0, self.reserve0, self.reserve1);
        msg::reply(
            PairEvent::Sync {
                reserve0: self.reserve0,
                reserve1: self.reserve1,
            },
            0,
        )
        .expect("PAIR: Error during a replying with PairEvent::Sync");
    }

    fn _mint_fee(&mut self, _reserve0: u128, _reserve1: u128) -> bool {
        // get feeTo address
        let feeTo: ActorId = ActorId::zero();
        let feeOn = false;
        if feeTo != ZERO_ID {
            feeOn = true;
        }
        if feeOn {
            if self.k_last != 0 {
                let root_k = self.reserve0.overflowing_mul(self.reserve1).0.sqrt();
                let root_k_last = self.k_last.sqrt();
                if root_k > root_k_last {
                    let numerator = self.get().total_supply.overflowing_mul(
                        root_k.overflowing_sub(root_k_last).0
                    ).0;
                    let denominator = root_k.overflowing_mul(5).0.overflowing_add(root_k_last).0;
                    let liquidity = numerator.overflowing_div(denominator).0;
                    if liquidity > 0 {
                        // _mint(feeTo, liquidity);
                    }
                }
            }
        } else {
            self.k_last = 0;
        }
        feeOn
    }

    fn _update(&mut self, balance0: u128, balance1: u128, _reserve0: u128, _reserve1: u128) {
        let current_ts = get_epoch_ms() % (2 << 32);
        let elapsed = self.last_block_ts - current_ts;
        if elapsed > 0 && _reserve0 != 0 && _reserve1 != 0 {
            // perform math
        }
        self.reserve0 = balance0;
        self.reserve1 = balance1;
        self.last_block_ts = current_ts;
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
        token_0: config.token_0,
        token_1: config.token_1,
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
    }
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let state: PairStateQuery = msg::load().expect("Unable to decode PairStateQuery");
    let pair = unsafe { PAIR.get_or_insert(Default::default()) };
    let reply = match state {

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
