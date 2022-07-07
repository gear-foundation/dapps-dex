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

    }
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairEvent {
    AddedLiquidity {
        amount0: u128,
        amount1: u128,
        to: ActorId,
    },
    RemovedLiquidity {

    }
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateQuery {

}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateReply {

}