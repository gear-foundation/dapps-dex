use dex_factory_io::*;
use gstd::{prelude::*, ActorId};
use gtest::{Program, RunResult, System};
use dex_factory_state::WASM_BINARY;

pub const USER: u64 = 10;
pub const FEE_SETTER: u64 = 11;

pub fn init_factory(sys: &System) {
    sys.init_logger();
    let factory = Program::current(sys);

    let res = factory.send(
        USER,
        InitFactory {
            fee_to_setter: ActorId::from(FEE_SETTER),
            pair_code_hash: [0; 32],
        },
    );
    assert!(res.log().is_empty());
}

pub fn create_pair(factory: &Program, user: u64, token_a: ActorId, token_b: ActorId) -> RunResult {
    factory.send(user, FactoryAction::CreatePair(token_a, token_b))
}

pub fn fee_to(factory: &Program, user: u64) -> RunResult {
    factory.send(user, FactoryAction::FeeTo)
}

pub fn set_fee_to(factory: &Program, user: u64, fee_to: ActorId) -> RunResult {
    factory.send(user, FactoryAction::SetFeeTo(fee_to))
}

pub fn set_fee_to_setter(factory: &Program, user: u64, fee_to_setter: ActorId) -> RunResult {
    factory.send(user, FactoryAction::SetFeeToSetter(fee_to_setter))
}

pub fn check_fee_to(factory: &Program, fee_to: ActorId) {
    match factory.read_state_using_wasm::<(), ActorId>("fee_to", WASM_BINARY.into(), None) {
        Ok(true_fee_to) => {
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

pub fn check_fee_to_setter(factory: &Program, fee_to_setter: ActorId) {
    match factory.read_state_using_wasm::<(), ActorId>("fee_to_setter", WASM_BINARY.into(), None) {
        Ok(true_fee_to_setter) => {
            if true_fee_to_setter != fee_to_setter {
                panic!("FACTORY: Actual fee_to_setter is different");
            }
        }
        _ => {
            unreachable!("Unreachable metastate reply for the FactoryStateQuery::FeeToSetter payload has occured")
        }
    }
}

pub fn check_pair_len(factory: &Program, length: u32) {
    match factory.read_state_using_wasm::<(), u32>("all_pairs_length", WASM_BINARY.into(), None) {
        Ok(true_length) => {
            if true_length != length {
                panic!("FACTORY: Actual length is different");
            }
        }
        _ => {
            unreachable!("Unreachable metastate reply for the FactoryStateQuery::AllPairsLength payload has occured")
        }
    }
}
