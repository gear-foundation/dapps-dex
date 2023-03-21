#![no_std]

use gear_lib::{
    fungible_token::{FTApproval, FTError, FTState, FTTransfer, FungibleToken},
    StorageProvider,
};
use gmeta::{InOut, Metadata};
use gstd::{errors::ContractError, prelude::*, ActorId};
use primitive_types::U256;

pub const MINIMUM_LIQUIDITY: u64 = 10u64.pow(3);

pub struct ContractMetadata;

impl Metadata for ContractMetadata {
    type Init = InOut<(ActorId, ActorId), Result<(), Error>>;
    type Handle = InOut<Action, Result<Event, Error>>;
    type Reply = ();
    type Others = ();
    type Signal = ();
    type State = State;
}

#[derive(
    Default,
    Encode,
    Decode,
    TypeInfo,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Hash,
    StorageProvider,
)]
pub struct State {
    pub factory: ActorId,

    pub token: (ActorId, ActorId),
    pub reserve: (u128, u128),
    pub cumulative_price: (U256, U256),

    pub last_block_ts: u64,
    pub k_last: U256,

    #[storage_field]
    pub ft_state: FTState,

    pub cached_actions: Vec<(ActorId, CachedAction)>,
}

impl FungibleToken for State {
    fn reply_transfer(&self, _transfer: FTTransfer) -> Result<(), FTError> {
        Ok(())
    }

    fn reply_approval(&self, _approval: FTApproval) -> Result<(), FTError> {
        Ok(())
    }
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum InnerAction {
    AddLiquidity {
        amount_a_desired: u128,
        amount_b_desired: u128,
        amount_a_min: u128,
        amount_b_min: u128,
        to: ActorId,
        deadline: u64,
    },

    RemoveLiquidity {
        liquidity: U256,
        amount_a_min: u128,
        amount_b_min: u128,
        to: ActorId,
        deadline: u64,
    },

    SwapExactTokensForTokens {
        amount_in: u128,
        amount_out_min: u128,
        to: ActorId,
        deadline: u64,
        swap_kind: SwapKind,
    },

    SwapTokensForExactTokens {
        amount_out: u128,
        amount_in_max: u128,
        to: ActorId,
        deadline: u64,
        swap_kind: SwapKind,
    },

    Skim(ActorId),

    Sync,

    Transfer {
        to: ActorId,
        amount: U256,
    },
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Action {
    pub action: InnerAction,
    pub kind: TransactionKind,
}

impl Action {
    pub fn new(action: InnerAction) -> Self {
        Self {
            action,
            kind: TransactionKind::New,
        }
    }

    pub fn to_retry(self) -> Self {
        Self {
            action: self.action,
            kind: TransactionKind::Retry,
        }
    }
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Event {
    AddedLiquidity {
        sender: ActorId,
        amount_a: u128,
        amount_b: u128,
        liquidity: U256,
    },
    RemovedLiquidity {
        sender: ActorId,
        amount_a: u128,
        amount_b: u128,
        to: ActorId,
    },
    Swap {
        sender: ActorId,
        in_amount: u128,
        out_amount: u128,
        to: ActorId,
        kind: SwapKind,
    },
    Sync {
        reserve_a: u128,
        reserve_b: u128,
    },
    Skim {
        amount_a: u128,
        amount_b: u128,
        to: ActorId,
    },
    Transfer {
        from: ActorId,
        to: ActorId,
        amount: U256,
    },
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum SwapKind {
    AForB,
    BForA,
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Error {
    ContractError(String),
    InsufficientAmount,
    InsufficientFormerAmount,
    InsufficientLatterAmount,
    InsufficientLiquidity,
    InvalidRecipient,
    ZeroActorId,
    TransferFailed,
    Overflow,
    DeadlineExceeded,
    FeeToGettingFailed,
    TxCacheError(TransactionCacheError),
}

impl From<ContractError> for Error {
    fn from(error: ContractError) -> Self {
        Self::ContractError(error.to_string())
    }
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum CachedAction {
    Swap(u128),
    AddLiquidity((u128, u128)),
    RemovedLiquidity(U256),
    Other,
}

#[derive(
    Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub enum TransactionKind {
    #[default]
    New,
    Retry,
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum TransactionCacheError {
    TransactionNotFound,
    MismatchedAction,
    StepOverflow,
}

impl From<TransactionCacheError> for Error {
    fn from(error: TransactionCacheError) -> Self {
        Self::TxCacheError(error)
    }
}

#[doc(hidden)]
pub mod hidden {
    use super::*;

    pub fn quote(amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
        if let Err(error) = perform_precalculate_check(amount, reserve) {
            Err(error)
        } else {
            let U256PairTuple(reserve) = reserve.into();

            if let Ok(result) = (U256::from(amount) * reserve.1 / reserve.0).try_into() {
                Ok(result)
            } else {
                Err(Error::Overflow)
            }
        }
    }

    pub fn calculate_out_amount(in_amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
        perform_precalculate_check(in_amount, reserve)?;

        let amount_with_fee: U256 = U256::from(in_amount) * 997;
        if let Some(numerator) = amount_with_fee.checked_mul(reserve.1.into()) {
            // Shouldn't overflow.
            let denominator = U256::from(reserve.0) * 1000 + amount_with_fee;

            // Shouldn't be more than u128::MAX, so casting doesn't lose data.
            Ok((numerator / denominator).low_u128())
        } else {
            Err(Error::Overflow)
        }
    }

    pub fn calculate_in_amount(out_amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
        perform_precalculate_check(out_amount, reserve)?;

        // The `u64` suffix is needed for a faster conversion.
        let numerator =
            (U256::from(reserve.0) * U256::from(out_amount)).checked_mul(1000u64.into());

        if let (Some(numerator), Some(amount)) = (numerator, reserve.1.checked_sub(out_amount)) {
            if amount == 0 {
                Err(Error::Overflow)
            } else {
                let denominator = U256::from(amount) * 997;

                // Adding 1 here to avoid abuse of the case when a calculated input
                // amount will equal 0.
                if let Ok(in_amount) = (numerator / denominator + 1).try_into() {
                    Ok(in_amount)
                } else {
                    Err(Error::Overflow)
                }
            }
        } else {
            Err(Error::Overflow)
        }
    }

    fn perform_precalculate_check(amount: u128, reserve: (u128, u128)) -> Result<(), Error> {
        if reserve.0 == 0 || reserve.1 == 0 {
            Err(Error::InsufficientLiquidity)
        } else if amount == 0 {
            Err(Error::InsufficientAmount)
        } else {
            Ok(())
        }
    }

    pub struct U256PairTuple(pub (U256, U256));

    impl From<(u128, u128)> for U256PairTuple {
        fn from(value: (u128, u128)) -> Self {
            Self((value.0.into(), value.1.into()))
        }
    }
}
