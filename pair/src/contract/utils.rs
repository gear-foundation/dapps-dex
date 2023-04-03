use super::tx_manager::TransactionGuard;
use dex_pair_io::*;
use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent};
use gstd::{
    errors::Result as GstdResult,
    msg::{self, CodecMessageFuture},
    prelude::*,
    ActorId,
};

pub fn send<T: Decode>(actor: ActorId, payload: impl Encode) -> GstdResult<CodecMessageFuture<T>> {
    msg::send_for_reply_as(actor, payload, 0)
}

pub async fn transfer_tokens<T>(
    tx_guard: &mut TransactionGuard<'_, T>,
    token: ActorId,
    sender: ActorId,
    recipient: ActorId,
    amount: u128,
) -> Result<(), Error> {
    let payload = FTokenAction::Message {
        transaction_id: tx_guard.step()?,
        payload: Action::Transfer {
            sender,
            recipient,
            amount,
        }
        .encode(),
    };

    if FTokenEvent::Ok != send(token, payload)?.await? {
        Err(Error::TransferFailed)
    } else {
        Ok(())
    }
}

pub async fn balance(token: ActorId, actor: ActorId) -> GstdResult<u128> {
    if let FTokenEvent::Balance(balance) = send(token, FTokenAction::GetBalance(actor))?.await? {
        Ok(balance)
    } else {
        unreachable!("received an unexpected `FTokenEvent` variant");
    }
}
