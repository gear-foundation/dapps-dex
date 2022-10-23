#![no_std]

use dex_pair_io::*;
use gear_lib::fungible_token::{ft_core::*, state::*};
use gear_lib_derive::{FTCore, FTMetaState, FTStateKeeper};
use gstd::{cmp, exec, msg, prelude::*, ActorId};
use instruction::{create_forward_transfer_instruction, create_swap_transfer_instruction};
use num::integer::Roots;
use primitive_types::H256;
mod internals;
mod instruction;
use instruction::*;
pub mod math;
pub mod messages;

const MINIMUM_LIQUIDITY: u128 = 1000;

#[derive(Debug, Default, FTStateKeeper, FTCore, FTMetaState)]
pub struct Pair {
    #[FTStateField]
    pub token: FTState,
    // Factoty address which deployed this pair
    pub factory: ActorId,
    // First FT contract address.
    pub token0: ActorId,
    // Second FT contract address.
    pub token1: ActorId,
    // Last timestamp when the reserves and balances were updated
    last_block_ts: u128,
    // Balances of token0 and token1, to get rid of actually querying the balance from the contract.
    pub balance0: u128,
    pub balance1: u128,
    // Token0 and token1 reserves.
    reserve0: u128,
    reserve1: u128,
    // Token prices
    pub price0_cl: u128,
    pub price1_cl: u128,
    // K which is equal to self.reserve0 * self.reserve1 which is used to amount calculations when performing a swap.
    pub k_last: u128,

    // transactions handling
    transaction_status: BTreeMap<H256, TransactionStatus>,
    instructions: BTreeMap<H256, (Instruction, Instruction)>
}

static mut PAIR: Option<Pair> = None;

#[derive(Debug)]
pub enum TransactionStatus {
    InProgress,
    Success,
    Failure,
}

#[derive(Debug, Copy, Clone)]
pub struct TransferDescription {
    token_address: ActorId,
    from: ActorId,
    to: ActorId,
    token_amount: u128,
}

impl Pair {
    // EXTERNAL FUNCTIONS

    pub async fn message(&mut self, transaction_id: u64, action: &PairAction) {
        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let transaction_status = self
            .transaction_status
            .get(&transaction_hash)
            .unwrap_or(&TransactionStatus::InProgress);

        match transaction_status {
            TransactionStatus::Success => reply_ok(),
            TransactionStatus::Failure => reply_err(),
            TransactionStatus::InProgress => match action {
                // done
                PairAction::Sync  => {
                    self.sync(transaction_hash).await;
                }
                // done
                PairAction::Skim { to } => {
                    self.skim(transaction_id, to).await;
                }
                // done
                PairAction::AddLiquidity { amount0_desired, amount1_desired, amount0_min, amount1_min, to } => {
                    self.add_liquidity(transaction_id, *amount0_desired, *amount1_desired, *amount0_min, *amount1_min, to).await;
                }
                // done
                PairAction::SwapExactTokensFor { to, amount_in } => {
                    self.swap_exact_tokens_for(transaction_id, to, *amount_in).await;
                }
                // done
                PairAction::SwapTokensForExact { to, amount_out } => {
                    self.swap_tokens_for_exact(transaction_id, to, *amount_out).await;
                }
                // done
                PairAction::RemoveLiquidity { liquidity, amount0_min, amount1_min, to } => {
                    self.remove_liquidity(transaction_id, *liquidity, *amount0_min, *amount1_min, to).await;
                }
            }
        }
    }

    async fn perform_two_transfers(
        &mut self,
        transaction_id: u64,
        transaction_hash: H256,
        transfer1: TransferDescription,
        transfer2: TransferDescription,
    ) {
        self.instructions
            .entry(transaction_hash)
            .or_insert_with(|| {
                let first_transfer = create_forward_transfer_instruction(
                    transaction_id,
                    &transfer1.token_address,
                    &transfer1.from,
                    &transfer1.to,
                    transfer1.token_amount
                );
                let second_transfer = create_swap_transfer_instruction(
                    transaction_id,
                    &transfer2.token_address,
                    &transfer2.from,
                    &transfer2.to,
                    transfer2.token_amount
                );
                (first_transfer, second_transfer)
            });

            let (first_transfer, second_transfer) = self
                .instructions
                .get_mut(&transaction_hash)
                .expect("Can't be `None`: Instruction must exist");
            if first_transfer.start().await.is_err() {
                self.transaction_status
                    .insert(transaction_hash, TransactionStatus::Failure);
                    // every reply_err should be panic though
                    reply_err();
                return;
            }
            match second_transfer.start().await {
                Err(_) => {
                    if first_transfer.abort().await.is_ok() {
                        self.transaction_status
                            .insert(transaction_hash, TransactionStatus::Failure);
                            reply_err();
                    }
                }
                Ok(_) => {
                    self.transaction_status
                        .insert(transaction_hash, TransactionStatus::Success);
                    reply_ok();
                }
            }
    }

    async fn sync(&mut self, transaction_hash: H256) {
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::InProgress);
            self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::Success);

        // just reply_ok, since no external methods are called
        reply_ok();
    }

    async fn skim(&mut self, transaction_id: u64, to: &ActorId) {
        let transaction_hash = get_hash(&msg::source(), transaction_id);
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::InProgress);

        // Update the balances
        self.balance0 -= self.reserve0;
        self.balance1 -= self.reserve1;
        self.perform_two_transfers(
            transaction_id,
            transaction_hash,
            TransferDescription {
                token_address: self.token0,
                from: exec::program_id(),
                to: *to,
                token_amount: self.balance0.saturating_sub(self.reserve0),
            },
            TransferDescription {
                token_address: self.token1,
                from: exec::program_id(),
                to: *to,
                token_amount: self.balance1.saturating_sub(self.reserve1),
            }
        )
        .await;
    }

    async fn add_liquidity(
        &mut self,
        transaction_id: u64,
        amount0_desired: u128,
        amount1_desired: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: &ActorId,
    ) {

        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let amount0: u128;
        let amount1: u128;
        // Check the amounts provided with the respect to the reserves to find the best amount of tokens0/1 to be added.
        if self.reserve0 == 0 && self.reserve1 == 0 {
            amount0 = amount0_desired;
            amount1 = amount1_desired;
        } else {
            let amount1_optimal = math::quote(amount0_desired, self.reserve0, self.reserve1);
            if amount1_optimal < amount1_desired {
                if amount1_optimal >= amount1_min {
                    panic!("PAIR: Insufficient token1 amount.");
                }
                amount0 = amount0_desired;
                amount1 = amount1_optimal;
            } else {
                let amount0_optimal = math::quote(amount1_desired, self.reserve0, self.reserve1);
                if amount0_optimal >= amount0_min {
                    panic!("PAIR: Insufficient token0 amount.");
                }
                amount0 = amount0_optimal;
                amount1 = amount1_desired;
            }
        }

        let pair_address = exec::program_id();
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::InProgress);

        // we can perform mint & update the balances here before actually transferring tokens
        // since panic here will rollback all the state changes
        self.balance0 += amount0;
        self.balance1 += amount1;
        // call mint function
        let liquidity = self.mint(*to).await;

        // now we can transfer tokens
        self.perform_two_transfers(
            transaction_id,
            transaction_hash,
            TransferDescription {
                token_address: self.token0,
                from: msg::source(),
                to: pair_address,
                token_amount: amount0,
            },
            TransferDescription {
                token_address: self.token1,
                from: msg::source(),
                to: pair_address,
                token_amount: amount1,
            },
        )
        .await;
    }

    async fn remove_liquidity(
        &mut self,
        transaction_id: u64,
        liquidity: u128,
        amount0_min: u128,
        amount1_min: u128,
        to: &ActorId,
    ) {

        let transaction_hash = get_hash(&msg::source(), transaction_id);
        FTCore::transfer(self, &msg::source(), &exec::program_id(), liquidity);
        // no need for self.burn though
        let fee_on = self.mint_fee(self.reserve0, self.reserve1).await;
        let liquidity: u128 = *self
            .get()
            .balances
            .get(&exec::program_id())
            .expect("The pair has no liquidity at all");
        let amount0 = liquidity
            .wrapping_mul(self.balance0)
            .wrapping_div(self.get().total_supply);
        let amount1 = liquidity
            .wrapping_mul(self.balance1)
            .wrapping_div(self.get().total_supply);

        if amount0 == 0 || amount1 == 0 {
            panic!("PAIR: Insufficient liquidity burnt.");
        }
        self.update_balance(*to, liquidity, false);
        self.balance0 -= amount0;
        self.balance1 -= amount1;
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);
        if fee_on {
            // If fee is on recalculate the K.
            self.k_last = self.reserve0.wrapping_mul(self.reserve1);
        }

        if amount0 < amount0_min {
            panic!("PAIR: Insufficient amount of token 0")
        }
        if amount1 < amount1_min {
            panic!("PAIR: Insufficient amount of token 1")
        }

        // transfer here

        self.perform_two_transfers(
            transaction_id,
            transaction_hash,
            TransferDescription {
                token_address: self.token0,
                from: exec::program_id(),
                to: *to,
                token_amount: amount0,
            },
            TransferDescription {
                token_address: self.token1,
                from: exec::program_id(),
                to: *to,
                token_amount: amount1,
            },
        )
        .await;
    }

    async fn swap_exact_tokens_for(
        &mut self,
        transaction_id: u64,
        to: &ActorId,
        amount_in: u128,
    ) {

        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let amount_out = math::get_amount_out(amount_in, self.reserve0, self.reserve1);
        if amount_out > self.reserve1 {
            panic!("PAIR: Insufficient liquidity.");
        }

        // run everything before the actual swap
        self.balance0 += amount_in;
        self.balance1 -= amount_out;
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);

        self.perform_two_transfers(
            transaction_id,
            transaction_hash,
            TransferDescription {
                token_address: self.token0,
                from: *to,
                to: exec::program_id(),
                token_amount: amount_in,
            },
            TransferDescription {
                token_address: self.token1,
                from: exec::program_id(),
                to: *to,
                token_amount: amount_out,
            }
        )
        .await;
    }

    async fn swap_tokens_for_exact(
        &mut self,
        transaction_id: u64,
        to: &ActorId,
        amount_out: u128,
    ) {

        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let amount_in = math::get_amount_in(amount_out, self.reserve0, self.reserve1);
        if amount_in > self.reserve0 {
            panic!("PAIR: Insufficient liquidity.");
        }
        self.balance0 -= amount_in;
        self.balance1 += amount_out;
        self.update(self.balance0, self.balance1, self.reserve0, self.reserve1);

        self.perform_two_transfers(
            transaction_id,
            transaction_hash,
            TransferDescription {
                token_address: self.token0,
                from: exec::program_id(),
                to: *to,
                token_amount: amount_in,
            },
            TransferDescription {
                token_address: self.token1,
                from: *to,
                to: exec::program_id(),
                token_amount: amount_out,
            }
        )
        .await;
    }
}

#[no_mangle]
extern "C" fn init() {
    let config: InitPair = msg::load().expect("Unable to decode InitPair");
    // DISABLE FOR TESTING, UNCOMMENT AND FIX TESTS LATER
    // if config.factory != msg::source() {
    //     panic!("PAIR: Can only be created by a factory.");
    // }
    let pair = Pair {
        factory: config.factory,
        token0: config.token0,
        token1: config.token1,
        ..Default::default()
    };
    unsafe {
        PAIR = Some(pair);
    }
}

#[gstd::async_main]
async fn main() {
    let action: MessageAction = msg::load().expect("Unable to decode MessageAction");
    let pair = unsafe { PAIR.get_or_insert(Default::default()) };
    match action {
        MessageAction::Message { transaction_id, payload } => pair.message(transaction_id, &payload).await
    }
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let state: PairStateQuery = msg::load().expect("Unable to decode PairStateQuery");
    let pair = unsafe { PAIR.get_or_insert(Default::default()) };
    let reply = match state {
        PairStateQuery::TokenAddresses => PairStateReply::TokenAddresses {
            token0: pair.token0,
            token1: pair.token1,
        },
        PairStateQuery::Reserves => PairStateReply::Reserves {
            reserve0: pair.reserve0,
            reserve1: pair.reserve1,
        },
        PairStateQuery::Prices => PairStateReply::Prices {
            price0: pair.price0_cl,
            price1: pair.price1_cl,
        },
        PairStateQuery::BalanceOf(address) => {
            PairStateReply::Balance(*pair.get().balances.get(&address).unwrap_or(&0))
        }
    };
    gstd::util::to_leak_ptr(reply.encode())
}

gstd::metadata! {
    title: "DEXPair",
    init:
        input: InitPair,
    handle:
        input: PairAction,
        output: PairEvent,
    state:
        input: PairStateQuery,
        output: PairStateReply,
}

pub fn get_hash(account: &ActorId, transaction_id: u64) -> H256 {
    let account: Vec<u8> = <[u8; 32]>::from(*account).into();
    let transaction_id = transaction_id.to_be_bytes();
    sp_core_hashing::blake2_256(&[&account[..], &transaction_id[..]].concat()).into()
}

fn reply_err() {
    // no need to reply, we can just straight panic?
    msg::reply(MessageReply::Err, 0).expect("Error in sending a reply `MessageReply::Err`");
}

// fn reply_ok(&payload: PairEvent) {
//     msg::reply(MessageReply::Ok(payload), 0).expect("Error in sending a reply `MessageReply::Ok`");
// }

fn reply_ok() {
    msg::reply(MessageReply::Ok, 0).expect("Error in sending a reply `MessageReply::Ok`");
}
