use crate::H256;
use gstd::{msg, ActorId};
use ft_io::*;


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
    transaction: FTAction,
    compensation: Option<FTAction>,
}

impl Instruction {
    pub fn new(
        address: ActorId,
        transaction: FTAction,
        compensation: Option<FTAction>
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
                msg::send_for_reply_as::<_, FTEvent>(self.address, self.transaction, 0)
                    .expect("Error in sending a message in instruction")
                    .await;
                self.state = InstructionState::ScheduledAbort;
                Ok(())
            }
            InstructionState::RunWithError => Err(()),
            _ => Ok(())
        }
    }

    pub async fn abort(&mut self) -> Result<(), ()> {
        match self.state {
            InstructionState::ScheduledAbort => {
                msg::send_for_reply_as::<_, FTEvent>(
                    self.address,
                    self.compensation
                        .as_ref()
                        .expect("No compensation for that instruction"),
                        0,
                )
                .expect("Error in sending a compensation message in instruction")
                .await;
                self.state = InstructionState::Finished;
                Ok(())
            }
            InstructionState::Finished => Ok(()),
            _ => Err(())
        }
    }
}

pub fn create_forward_transfer_instruction(
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTAction::Transfer {
            from: *from,
            to: *to,
            amount: token_amount,
        },
        Some(FTAction::Transfer {
            from: *to,
            to: *from,
            amount: token_amount,
        }),
    )
}

pub fn create_swap_transfer_instruction(
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTAction::Transfer {
            from: *from,
            to: *to,
            amount: token_amount,
        },
        None,
    )
}

pub fn create_approval_instruction(
    token_address: &ActorId,
    to: &ActorId,
    token_amount: u128,
) -> Instruction {
    Instruction::new(
        *token_address,
        FTAction::Approve {
            to: *to,
            amount: token_amount,
        },
        // No RevokeApproval for an FT implementation :)
        None,
    )
}
