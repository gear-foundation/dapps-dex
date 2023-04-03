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

/// The contract state.
///
/// For more info about fields, see [`Initialize`].
#[derive(Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct State {
    pub pair: CodeId,
    pub fee_to: ActorId,
    pub fee_to_setter: ActorId,
    pub pairs: Vec<((ActorId, ActorId), ActorId)>,
}

/// Initializes the contract.
#[derive(
    Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub struct Initialize {
    /// Actor that'll receive the 0.05% commission per trade.
    ///
    /// If it'll equal to [`ActorId::zero()`], the commission will be disabled.
    pub fee_to: ActorId,
    /// Actor that'll have the right to set `fee_to` & `fee_to_setter`.
    pub fee_to_setter: ActorId,
    /// Hashcode of the Pair contract.
    pub pair: CodeId,
}

/// Sends the contract info about what it should do.
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Action {
    /// Creates a Pair contract instance from a pair of
    /// (SFT)[https://github.com/gear-dapps/sharded-fungible-token]
    /// [`ActorId`]s.
    ///
    /// # Requirements:
    /// - [`ActorId`]s mustn't be identical.
    /// - [`ActorId`]s mustn't equal to [`ActorId::zero()`].
    /// - Pair with given [`ActorId`]s mustn't already exist.
    ///
    /// On success, replies with [`Event::PairCreated`].
    CreatePair(ActorId, ActorId),

    /// Sets [`ActorId`] of the fee receiver (`fee_to`).
    ///
    /// Setting the fee receiver to [`ActorId::zero()`] disables the 0.05%
    /// commission.
    ///
    /// # Requirements:
    /// - [`msg::source`](gstd::msg::source) must have the right to set the fee
    /// receiver (must be equal to `fee_to_setter`).
    ///
    /// On success, replies with [`Event::FeeToSet`].
    FeeTo(ActorId),

    /// Sets [`ActorId`] that'll have the right to set `fee_to` &
    /// `fee_to_setter`.
    ///
    /// # Requirements:
    /// - [`msg::source`](gstd::msg::source) must be equal to current
    /// `fee_to_setter`.
    ///
    /// On success, replies with [`Event::FeeToSetterSet`].
    FeeToSetter(ActorId),

    /// Gets [`ActorId`] of the current fee receiver.
    ///
    /// If it equals [`ActorId::zero()`], it means that the 0.05% commission is
    /// disabled.
    ///
    /// On success, replies with [`Event::FeeToSet`].
    GetFeeTo,
}

/// A result of successfully processed [`Action`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Event {
    /// Should be returned from [`Action::CreatePair`].
    PairCreated {
        /// A pair of SFT [`ActorId`]s.
        token_pair: (ActorId, ActorId),
        /// [`ActorId`] of a created Pair contract.
        pair_actor: ActorId,
        /// A number of Pair contracts (including a created one) inside the
        /// Factory contract.
        pair_number: u32,
    },

    /// Should be returned from [`Action::FeeToSetter`].
    FeeToSetterSet(
        /// New `fee_to_setter`.
        ActorId,
    ),

    /// Should be returned from [`Action::FeeTo`].
    FeeToSet(
        /// New `fee_to`.
        ActorId,
    ),
}

/// Error variants of failed [`Action`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Error {
    /// See [`ContractError`].
    ContractError(String),
    /// [`msg::source()`](gstd::msg::source) doesn't equal to `fee_to_setter`.
    AccessRestricted,
    /// [`ActorId::zero()`] was found where it's forbidden.
    ZeroActorId,
    /// SFT [`ActorId`]s in a given pair for creating the Pair contract are
    /// equal.
    IdenticalTokens,
    /// A pair contract with given SFT [`ActorId`]s already exist.
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
