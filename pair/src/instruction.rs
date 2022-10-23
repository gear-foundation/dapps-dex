use ft_logic_io::*;
use ft_main_io::*;
use gstd::{msg, prelude::*, ActorId};

#[derive(Debug)]
pub enum InstructionState {
    ScheduledRun,
    ScheduledAbort,
    RunWithError,
    Finished,
}

#[derive(Debug)]
pub struct Instruction {
    state: InstructionState,
    address: ActorId,
    transaction: FTokenAction,
    compensation: Option<FTokenAction>,
}

impl Instruction {
    pub fn new(
        address: ActorId,
        transaction: FTokenAction,
        compensation: Option<FTokenAction>,
    ) -> Self {
        Instruction {
            state: InstructionState::ScheduledRun,
            address,
            transaction,
            compensation,
        }
    }

    pub async fn start(&mut self) -> Result<(), ()> {
        match self.state {
            InstructionState::ScheduledRun => {
                // Right now it's FTokenEvent, but should be moved
                // LACKING CLONE OR COPY HERE :)
                let result =
                    msg::send_for_reply_as::<_, FTokenEvent>(self.address, self.transaction, 0)
                        .expect("Error in sending a message in instruction")
                        .await;
                match result {
                    Ok(FTokenEvent::Ok) => {
                        self.state = InstructionState::ScheduledAbort;
                        Ok(())
                    }
                    _ => {
                        self.state = InstructionState::RunWithError;
                        Err(())
                    }
                }
            }
            InstructionState::RunWithError => Err(()),
            _ => Ok(()),
        }
    }

    pub async fn abort(&mut self) -> Result<(), ()> {
        match self.state {
            InstructionState::ScheduledAbort => {
                let result = msg::send_for_reply_as::<_, FTokenEvent>(
                    self.address,
                    self.compensation
                        .as_ref()
                        .expect("No compensation for that instruction"),
                    0,
                )
                .expect("Error in sending a compensation message in instruction")
                .await;
                match result {
                    Ok(FTokenEvent::Ok) => {
                        self.state = InstructionState::Finished;
                        Ok(())
                    }
                    _ => Err(()),
                }
            }
            InstructionState::Finished => Ok(()),
            _ => Err(()),
        }
    }
}

pub fn create_forward_transfer_instruction(
    transaction_id: u64,
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTokenAction::Message {
            transaction_id,
            payload: Action::Transfer {
                sender: *from,
                recipient: *to,
                amount: token_amount,
            }
            .encode(),
        },
        Some(FTokenAction::Message {
            transaction_id,
            payload: Action::Transfer {
                sender: *to,
                recipient: *from,
                amount: token_amount,
            }
            .encode(),
        }),
    )
}

pub fn create_swap_transfer_instruction(
    transaction_id: u64,
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTokenAction::Message {
            transaction_id,
            payload: Action::Transfer {
                sender: *from,
                recipient: *to,
                amount: token_amount,
            }
            .encode(),
        },
        None,
    )
}

pub fn create_approval_instruction(
    transaction_id: u64,
    token_address: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTokenAction::Message {
            transaction_id,
            payload: Action::Approve {
                approved_account: *to,
                amount: token_amount,
            }
            .encode(),
        },
        // No RevokeApproval for an FT implementation :)
        None,
    )
}
