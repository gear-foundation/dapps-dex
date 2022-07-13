use ft_io::*;
use factory_io::*;
use gstd::{msg, ActorId};

pub async fn get_fee_to(factory_address: &ActorId) -> ActorId {
    let fee_to_response: FactoryEvent =
        msg::send_for_reply_as(*factory_address, FactoryAction::FeeTo, 0)
            .unwrap()
            .await
            .expect("Error in get_fee_to");
    if let FactoryEvent::FeeTo{ address: fee_to } = fee_to_response {
        return fee_to;
    }
    ActorId::zero()
}

pub async fn transfer_tokens(
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    token_amount: u128,
) {
    msg::send_for_reply_as::<_, FTEvent>(
        *token_address,
        FTAction::Transfer {
            from: *from,
            to: *to,
            amount: token_amount,
        },
        0,
    )
    .unwrap()
    .await
    .expect("Error in transfer");
}

pub async fn get_balance(token_address: &ActorId, account: &ActorId) -> u128 {
    let balance_response: FTEvent =
        msg::send_for_reply_as(*token_address, FTAction::BalanceOf(*account), 0)
            .unwrap()
            .await
            .expect("Error in balanceOf");
    if let FTEvent::Balance(balance_response) = balance_response {
        return balance_response;
    }
    0
}
