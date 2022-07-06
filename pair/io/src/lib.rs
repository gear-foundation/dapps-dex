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
    Sync {},
    Skim {
        to: ActorId,
    },
    Mint {
        to: ActorId,
    },
    Burn {
        to: ActorId,
    },
    Swap {
        amount0: u128,
        amount1: u128,
        to: ActorId,
    }
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairEvent {
    Sync {
        reserve0: u128,
        reserve1: u128,
    },
    Skim {
        to: ActorId,
    },
    Mint {
        to: ActorId,
        amount0: u128,
        amount1: u128,
    },
    Burn {
        sender: ActorId,
        to: ActorId,
        amount0: u128,
        amount1: u128,
    },
    Swap {
        sender: ActorId,
        amount0_in: u128,
        amount0_out: u128,
        amount1_in: u128,
        amount1_out: u128,
        to: ActorId,
    },
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateQuery {

}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateReply {

}