#![no_std]
use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct InitFactory {
    pub fee_to_setter: ActorId,
    pub pair_code_hash: [u8; 32],
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryAction {
    CreatePair { token_a: ActorId, token_b: ActorId },
    SetFeeTo { fee_to: ActorId },
    SetFeeToSetter { fee_to_setter: ActorId },
    FeeTo,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryEvent {
    PairCreated {
        token_a: ActorId,
        token_b: ActorId,
        pair_address: ActorId,
        pairs_length: u32,
    },
    FeeToSet {
        fee_to: ActorId,
    },
    FeeToSetterSet {
        fee_to_setter: ActorId,
    },
    FeeTo {
        address: ActorId,
    },
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum FactoryStateQuery {
    FeeTo,
    FeeToSetter,
    PairAddress { token_a: ActorId, token_b: ActorId },
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
