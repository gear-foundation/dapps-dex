#![no_std]

use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

pub type TokenId = ActorId;

/// Initializes a pair.
///
/// # Requirements:
/// * both `TokenId` MUST be fungible token contracts with a non-zero address.
/// * factory MUST be a non-zero address.
#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct InitPair {
    /// Factory address which deployed this pair.
    pub factory: ActorId,
    /// The first FT token address.
    pub token0: TokenId,
    /// The second FT token address.
    pub token1: TokenId,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairAction {
    /// Adds liquidity to the pair.
    ///
    /// Adds a specified amount of both tokens to the pair contract.
    /// # Requirements:
    /// * all the values MUST non-zero positive numbers.
    /// * `to` MUST be a non-zero adddress.
    ///
    /// On success returns `PairEvent::AddedLiquidity`.
    AddLiquidity {
        /// The amount of token 0 which is desired by a user.
        amount0_desired: u128,
        /// The amount of token 1 which is desired by a user.
        amount1_desired: u128,
        /// The minimum amount of token 0 which a user is willing to add.
        amount0_min: u128,
        /// The minimum amount of token 1 which a user is willing to add.
        amount1_min: u128,
        /// Who is adding the liquidity.
        to: ActorId,
    },

    /// Removes liquidity from the pair.
    ///
    /// Removes a specified amount of liquidity from the pair contact.
    /// # Requirements:
    /// * all the values MUST non-zero positive numbers.
    /// * `to` MUST be a non-zero adddress.
    ///
    /// On success returns `PairEvent::RemovedLiquidity`.
    RemoveLiquidity {
        /// Liquidity amount to be removed.
        liquidity: u128,
        /// The minimal amount of token 0 a user is willing to get.
        amount0_min: u128,
        /// The minimal amount of token 1 a user is willing to get.
        amount1_min: u128,
        // Who is removing liquidity.
        to: ActorId,
    },

    /// Forces the reserves to match the balances.
    ///
    /// On success returns `PairEvent::Sync`.
    Sync,

    /// Forces the reserves to match the balances.
    ///
    /// Forces the reserves to match the balances while sending all the extra tokens to a specified user.
    /// On success returns `PairEvent::Skim`
    Skim {
        /// Who will get extra tokens.
        to: ActorId,
    },

    /// Swaps token 0 for token 1.
    ///
    /// Swaps the provided amount of token 0 for token 1.
    /// Requirements:
    /// * `to` - MUST be a non-zero address.
    /// * `amount_in` - MUST be a positive number and less than the liquidity of token 0.
    ///
    /// On success returns `PairEvent::SwapExactTokensFor`.
    SwapExactTokensFor {
        /// Who is performing a swap.
        to: ActorId,
        /// Amount of token0 you wish to trade.
        amount_in: u128,
    },

    /// Swaps token 1 for token 0.
    ///
    /// Swaps the provided amount of token 1 for token 0.
    /// Requirements:
    /// * `to` - MUST be a non-zero address.
    /// * `amount_out` - MUST be a positive number and less than the liquidity of token 1.
    ///
    /// On sucess returns `PairEvent::SwapTokensForExact`.
    SwapTokensForExact {
        /// Who is performing a swap.
        to: ActorId,
        /// Amount of token 0 the user with to trade.
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
    Sync {
        balance0: u128,
        balance1: u128,
        reserve0: u128,
        reserve1: u128,
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
    TokenAddresses,
    Reserves,
    Prices,
    BalanceOf(ActorId),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum PairStateReply {
    TokenAddresses { token0: TokenId, token1: TokenId },
    Reserves { reserve0: u128, reserve1: u128 },
    Prices { price0: u128, price1: u128 },
    Balance(u128),
}
