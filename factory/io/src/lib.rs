#![no_std]

use gmeta::{InOut, Metadata};
use gstd::{errors::ContractError, prelude::*, ActorId, CodeId};

pub struct ContractMetadata;

impl Metadata for ContractMetadata {
    type Init = InOut<Initialize, Result<(), Error>>;
    type Handle = InOut<Action, Result<Event, Error>>;
    type Reply = ();
    type Others = ();
    type Signal = ();
    type State = State;
}

#[derive(Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct State {
    pub pair: CodeId,
    pub fee_to: ActorId,
    pub fee_to_setter: ActorId,
    pub pairs: Vec<((ActorId, ActorId), ActorId)>,
}

#[derive(
    Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub struct Initialize {
    pub fee_to: ActorId,
    pub fee_to_setter: ActorId,
    pub pair: CodeId,
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Action {
    CreatePair(ActorId, ActorId),

    FeeTo(ActorId),

    FeeToSetter(ActorId),

    GetFeeTo,
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Event {
    PairCreated {
        token_pair: (ActorId, ActorId),
        pair_actor: ActorId,
        pair_number: u32,
    },
    FeeToSetterSet(ActorId),
    FeeToSet(ActorId),
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Error {
    ContractError(String),
    AccessRestricted,
    ZeroActorId,
    IdenticalTokens,
    PairExist,
    PairCreationFailed(dex_pair_io::Error),
}

impl From<ContractError> for Error {
    fn from(error: ContractError) -> Self {
        Self::ContractError(error.to_string())
    }
}

impl From<dex_pair_io::Error> for Error {
    fn from(error: dex_pair_io::Error) -> Self {
        Self::PairCreationFailed(error)
    }
}
