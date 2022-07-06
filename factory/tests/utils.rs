use factory_io::*;
use gstd::{prelude::*, ActorId};
use gtest::{Program, RunResult, System};
pub const USER: u64 = 10;
pub const FEE_SETTER: u64 = 11;
pub const NEW_FEE_SETTER: u64 = 12;
pub const NEW_FEE_TO: u64 = 13;
pub const ZERO_ID: ActorId = ActorId::zero();
pub const TOKEN_A: u64 = 101;
pub const TOKEN_B: u64 = 102;

pub fn init_factory(sys: &System) {
    sys.init_logger();
    let factory = Program::current(sys);

    let res = factory.send(
        USER,
        InitFactory {
            fee_to_setter: ActorId::from(FEE_SETTER),
        },
    );
    assert!(res.log().is_empty());
}

pub fn create_pair_utils(
    factory: &Program,
    user: u64,
    token_a: ActorId,
    token_b: ActorId,
) -> RunResult {
    factory.send(user, FactoryAction::CreatePair { token_a, token_b })
}

pub fn set_fee_to_utils(factory: &Program, user: u64, fee_to: ActorId) -> RunResult {
    factory.send(user, FactoryAction::SetFeeTo { fee_to })
}

pub fn set_fee_to_setter_utils(factory: &Program, user: u64, fee_to_setter: ActorId) -> RunResult {
    factory.send(user, FactoryAction::SetFeeToSetter { fee_to_setter })
}

pub fn check_fee_to(factory: &Program, user: u64, fee_to: ActorId) {
    match factory.meta_state(FactoryStateQuery::FeeTo {}) {
        gstd::Ok(FactoryStateReply::FeeTo {
            address: true_fee_to,
        }) => {
            if true_fee_to != fee_to {
                panic!("FACTORY: Actual fee_to is different");
            }
        }
        _ => {
            unreachable!(
                "Unreachable metastate reply for the FactoryStateQuery::FeeTo payload has occured"
            )
        }
    }
}

pub fn check_fee_to_setter(factory: &Program, user: u64, fee_to_setter: ActorId) {
    match factory.meta_state(FactoryStateQuery::FeeToSetter {}) {
        gstd::Ok(FactoryStateReply::FeeToSetter {
            address: true_fee_to_setter,
        }) => {
            if true_fee_to_setter != fee_to_setter {
                panic!("FACTORY: Actual fee_to_setter is different");
            }
        }
        _ => {
            unreachable!("Unreachable metastate reply for the FactoryStateQuery::FeeToSetter payload has occured")
        }
    }
}

pub fn check_pair_len(factory: &Program, user: u64, length: u32) {
    match factory.meta_state(FactoryStateQuery::AllPairsLength {}) {
        gstd::Ok(FactoryStateReply::AllPairsLength {
            length: true_length,
        }) => {
            if true_length != length {
                panic!("FACTORY: Actual length is different");
            }
        }
        _ => {
            unreachable!("Unreachable metastate reply for the FactoryStateQuery::AllPairsLength payload has occured")
        }
    }
}