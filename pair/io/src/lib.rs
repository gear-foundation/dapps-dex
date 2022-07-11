#![no_std]
use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct InitPair {
    pub factory: ActorId,
    pub token_0: ActorId,
    pub token_1: ActorId,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairAction {
    AddLiquidity {
        amount0_desired: u128,
        amount1_desired: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    },
    RemoveLiquidity {
        liquidity: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: ActorId,
    },
    Sync {},
    Skim {
        to: ActorId,
    },
    SwapExactTokensFor {
        to: ActorId,
        amount_in: u128,
    },
    SwapTokensForExact {
        to: ActorId,
        amount_out: u128,
    },
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairEvent {
    AddedLiquidity {
        amount0: u128,
        amount1: u128,
        liquidity: u128,
        to: ActorId,
    },
    RemovedLiquidity {
        liquidity: u128,
        to: ActorId,
    },
    Sync {
        balance0: u128,
        balance1: u128,
        reserve0: u128,
        reverve1: u128,
    },
    Skim {
        to: ActorId,
        amount0: u128,
        amount1: u128,
    },
    SwapExactTokensFor {
        to: ActorId,
        amount_in: u128,
        amount_out: u128,
    },
    SwapTokensForExact {
        to: ActorId,
        amount_in: u128,
        amount_out: u128,
    },
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateQuery {
    TokenAddresses {},
    Reserves {},
    Prices {},
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateReply {
    TokenAddresses { token0: ActorId, token1: ActorId },
    Reserves { reserve0: u128, reserve1: u128 },
    Prices { price0: u128, price1: u128 },
}
