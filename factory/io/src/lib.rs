#![no_std]
use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

pub type TokenId = ActorId;

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct InitFactory {
    pub fee_to_setter: ActorId,
    pub pair_code_hash: [u8; 32],
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryAction {
    CreatePair(ActorId, ActorId),
    SetFeeTo(ActorId),
    SetFeeToSetter(ActorId),
    FeeTo,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryEvent {
    PairCreated {
        token_a: TokenId,
        token_b: TokenId,
        pair_address: ActorId,
        pairs_length: u32,
    },
    FeeToSet(ActorId),
    FeeToSetterSet(ActorId),
    FeeTo(ActorId),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryStateQuery {
    FeeTo,
    FeeToSetter,
    PairAddress { token_a: TokenId, token_b: TokenId },
    AllPairsLength,
    Owner,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryStateReply {
    FeeTo { address: ActorId },
    FeeToSetter { address: ActorId },
    PairAddress { address: ActorId },
    AllPairsLength { length: u32 },
    Owner { address: ActorId },
}
