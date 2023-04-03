#![no_std]

use gear_lib::{
    fungible_token::{FTApproval, FTError, FTState, FTTransfer, FungibleToken},
    StorageProvider,
};
use gmeta::{InOut, Metadata};
use gstd::{errors::ContractError, prelude::*, ActorId};
use primitive_types::U256;

/// The minimal liquidity amount.
///
/// To ameliorate rounding errors and increase the theoretical minimum tick size
/// for liquidity provision, the contract burns this amount of liquidity tokens
/// on the first mint (first [`InnerAction::AddLiquidity`]).
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

/// The contract state.
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
    /// [`ActorId`] of the Factory contract from which [`ActorId`] of the fee
    /// receiver (`fee_to`) is obtained.
    pub factory: ActorId,

    /// The pair of SFT [ActorId]s that are used for swaps.
    pub token: (ActorId, ActorId),
    /// The record of tokens reserve in the SFT pair (`token`).
    pub reserve: (u128, u128),
    /// https://docs.uniswap.org/contracts/v2/concepts/core-concepts/oracles
    pub cumulative_price: (U256, U256),

    /// A timestamp of the last block where `reserve`s were changed.
    pub last_block_ts: u64,
    /// A product of `reserve`s. Used for the 0.05% commission calculation.
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

/// A part of [`Action`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum InnerAction {
    /// Adds liquidity to the contract from [`msg::source()`]'s fungible tokens
    /// and mints liquidity tokens to it.
    ///
    /// # Requirements
    /// - `amount_a_desired` & `amount_b_desired` mustn't equal to 0.
    /// - On the first addition (first mint), a resulted amount of pool tokens
    /// must be more than [`MINIMUM_LIQUIDITY`].
    AddLiquidity {
        /// An amount of the A tokens to add as liquidity if the B/A price is <=
        /// `amount_b_desired`/`amount_a_desired` (A depreciates).
        amount_a_desired: u128,
        /// An amount of the B tokens to add as liquidity if the A/B price is <=
        /// `amount_a_desired`/`amount_b_desired` (B depreciates).
        amount_b_desired: u128,
        /// Bounds an extent to which the B/A price can go up before this action
        /// reverts. Must be <= `amount_a_desired`.
        amount_a_min: u128,
        /// Bounds an extent to which the A/B price can go up before this action
        /// reverts. Must be <= `amount_b_desired`.
        amount_b_min: u128,
        /// A recipient of minted liquidity tokens.
        to: ActorId,
        /// Timestamp (in ms) after which this action will revert.
        deadline: u64,
    },

    /// Removes liquidity from the contract by burning [`msg::source()`]'s
    /// liquidity tokens and transferring an appropriate amount of fungible
    /// tokens from the contract to it.
    ///
    /// # Requirements
    /// - [`msg::source()`] must have the same or a greater amount of liquidity
    /// tokens than a given one.
    RemoveLiquidity {
        /// An amount of liquidity tokens to remove.
        liquidity: U256,
        /// A minimum amount of the A tokens that must be received for this
        /// action not to revert.
        amount_a_min: u128,
        /// A minimum amount of the B tokens that must be received for this
        /// action not to revert.
        amount_b_min: u128,
        /// A recipient of returned fungible tokens.
        to: ActorId,
        /// Timestamp (in ms) after which this action will revert.
        deadline: u64,
    },

    // Swaps an exact amount of input tokens for as many output tokens as
    // possible.
    ///
    /// # Requirements
    /// - `to` mustn't equal to the contract's SFT pair.
    /// - `amount_in` mustn't equal to 0.
    SwapExactTokensForTokens {
        amount_in: u128,
        /// A minimum amount of output tokens that must be received for this
        /// action not to revert.
        amount_out_min: u128,
        /// A recipient of output tokens.
        to: ActorId,
        /// Timestamp (in ms) after which this action will revert.
        deadline: u64,
        swap_kind: SwapKind,
    },

    /// Swaps an exact amount of output tokens for as few input tokens as
    /// possible.
    ///
    /// # Requirements
    /// - `to` mustn't equal to the contract's SFT pair.
    /// - `amount_out` mustn't equal to 0.
    SwapTokensForExactTokens {
        amount_out: u128,
        /// A maximum amount of input tokens that must be received for this
        /// action not to revert.
        amount_in_max: u128,
        /// A recipient of output tokens.
        to: ActorId,
        /// Timestamp (in ms) after which this action will revert.
        deadline: u64,
        swap_kind: SwapKind,
    },

    /// Syncs the contract's tokens reserve with actual contract's balances by
    /// transferring excess tokens to some [`ActorId`].
    Skim(
        /// A recipient of excess tokens.
        ActorId,
    ),

    /// Syncs the contract's tokens reserve with actual contract's balances by
    /// setting the reserve equal to the balances.
    Sync,

    /// Transfers liquidity tokens from [`msg::source()`].
    ///
    /// # Requirements
    /// - [`msg::source()`] must have the same or a greater amount of liquidity
    /// tokens than a given one.
    Transfer { to: ActorId, amount: U256 },
}

/// Sends the contract info about what it should do.
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

/// A result of successfully processed [`Action`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Event {
    /// Should be returned from [`InnerAction::AddLiquidity`].
    AddedLiquidity {
        sender: ActorId,
        /// An amount of the A token sent to the contract.
        amount_a: u128,
        /// An amount of the B token sent to the contract.
        amount_b: u128,
        /// An amount of liquidity tokens minted.
        liquidity: U256,
    },
    /// Should be returned from [`InnerAction::RemoveLiquidity`].
    RemovedLiquidity {
        sender: ActorId,
        /// An amount of the A token returned from the contract.
        amount_a: u128,
        /// An amount of the B token returned from the contract.
        amount_b: u128,
        /// A recipient of returned fungible tokens.
        to: ActorId,
    },
    /// Should be returned from
    /// [`InnerAction::SwapExactTokensForTokens`]/[`InnerAction::SwapTokensForExactTokens`].
    Swap {
        sender: ActorId,
        in_amount: u128,
        out_amount: u128,
        to: ActorId,
        kind: SwapKind,
    },
    /// Should be returned from [`InnerAction::Sync`].
    Sync {
        /// The current amount of the A token in the contract's reserve.
        reserve_a: u128,
        /// The current amount of the B token in the contract's reserve.
        reserve_b: u128,
    },
    /// Should be returned from [`InnerAction::Skim`].
    Skim {
        /// A skimmed amount of the A token.
        amount_a: u128,
        /// A skimmed amount of the A token.
        amount_b: u128,
        /// A recipient of skimmed tokens.
        to: ActorId,
    },
    /// Should be returned from [`InnerAction::Transfer`].
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

/// Error variants of failed [`Action`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Error {
    /// See [`ContractError`].
    ContractError(String),
    /// An insufficient amount of the A or B token was provided.
    InsufficientAmount,
    /// A specified amount limit of the former tokens has been exceeded.
    InsufficientFormerAmount,
    /// A specified amount limit of the latter tokens has been exceeded.
    InsufficientLatterAmount,
    /// An insufficient amount of liquidity tokens was provided, or the contract
    /// doesn't have enough of them to continue an action.
    InsufficientLiquidity,
    /// An invalid recipient was specified.
    InvalidRecipient,
    /// [`ActorId::zero()`] was found where it's forbidden.
    ZeroActorId,
    /// One of the contract's FT contracts failed to complete a transfer
    /// action.
    ///
    /// Most often, the reason is that a user didn't give an approval to the
    /// contract or didn't have enough tokens to transfer.
    TransferFailed,
    /// An overflow occurred during calculations.
    Overflow,
    /// A specified deadline for an action was exceeded.
    DeadlineExceeded,
    /// The contract failed to get fee receiver (`fee_to`) [`ActorId`] from the
    /// linked Factory contract.
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

/// A part of [`Action`].
///
/// Determines how an action will be processed.
///
/// The contract has a transaction caching mechanism for a continuation of
/// partially processed asynchronous actions. Most often, the reason of an
/// underprocession is the lack of gas.
///
/// Important notes:
/// - Only the last sent asynchronous action for
/// [`msg::source()`](gstd::msg::source) is cached.
/// - Non-asynchronous actions are never cached.
/// - There's no guarantee every underprocessed asynchronous action will be
/// cached.
/// - It's possible to send a retry action with a different payload, and it'll
/// continue with it because, for some action, not all payload is saved in the
/// cache.
/// - The cache memory has a limit, so when it's reached every oldest cached
/// action is replaced with a new one.
#[derive(
    Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub enum TransactionKind {
    #[default]
    New,
    Retry,
}

/// Transaction cache error variants.
///
/// Also see [`TransactionKind`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum TransactionCacheError {
    /// There's no cached transaction for
    /// [`msg::source()`](gstd::msg::source()). The reason may be a
    /// transaction's action wasn't asynchronous or just wasn't cached, or a
    /// cached transaction has been removed because it was completed or too old.
    TransactionNotFound,
    /// An action for retrying doesn't match its cached counterpart.
    MismatchedAction,
    /// Too many transaction IDs were acquired in one action. The maximum amount
    /// is 256.
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
            quote_unchecked(amount, reserve)
        }
    }

    pub fn quote_unchecked(amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
        let U256PairTuple(reserve) = reserve.into();

        if let Ok(result) = (U256::from(amount) * reserve.1 / reserve.0).try_into() {
            Ok(result)
        } else {
            Err(Error::Overflow)
        }
    }

    pub fn quote_reserve_unchecked(amount: u128, reserve: (u128, u128)) -> Result<u128, Error> {
        if amount == 0 {
            Err(Error::InsufficientAmount)
        } else {
            quote_unchecked(amount, reserve)
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

    pub fn perform_precalculate_check(amount: u128, reserve: (u128, u128)) -> Result<(), Error> {
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

    #[cfg(test)]
    mod tests {
        use super::{super::Error, calculate_in_amount, calculate_out_amount, quote_unchecked};

        #[test]
        fn quote() {
            assert_eq!(quote_unchecked(5000, (10000, 10000)), Ok(5000));
            assert_eq!(quote_unchecked(1234, (54321, 12345)), Ok(280));
        }

        #[test]
        fn calculate_oa() {
            assert_eq!(
                calculate_out_amount(0, (0, 1)),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                calculate_out_amount(0, (1, 0)),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                calculate_out_amount(0, (1, 1)),
                Err(Error::InsufficientAmount)
            );

            assert_eq!(
                calculate_out_amount(u128::MAX, (1, u128::MAX)),
                Err(Error::Overflow)
            );

            // (10000 * 997) * 10000 // (10000 * 1000 + (10000 * 997))
            assert_eq!(calculate_out_amount(10000, (10000, 10000)), Ok(4992));
            // (1234 * 997) * 54321 // (12345 * 1000 + (1234 * 997))
            assert_eq!(calculate_out_amount(1234, (12345, 54321)), Ok(4922));
        }

        #[test]
        fn calculate_ia() {
            assert_eq!(
                calculate_in_amount(0, (0, 1)),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                calculate_in_amount(0, (1, 0)),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                calculate_in_amount(0, (1, 1)),
                Err(Error::InsufficientAmount)
            );

            assert_eq!(
                calculate_in_amount(u128::MAX, (u128::MAX, 1)),
                Err(Error::Overflow)
            );
            // reserve.1 - out_amount == 0
            assert_eq!(calculate_in_amount(12345, (1, 12345)), Err(Error::Overflow));
            assert_eq!(
                calculate_in_amount(u128::MAX / 100 - 1, (u128::MAX / 100, u128::MAX / 100)),
                Err(Error::Overflow)
            );

            // 5000 * 10000 * 1000 // ((10000 - 5000) * 997) + 1
            assert_eq!(calculate_in_amount(5000, (10000, 10000)), Ok(10031));
            // 1234 * 12345 * 1000 // ((54321 - 1234) * 997) + 1
            assert_eq!(calculate_in_amount(1234, (12345, 54321)), Ok(288));
        }
    }
}
