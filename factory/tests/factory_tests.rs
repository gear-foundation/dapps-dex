use factory_io::*;
use gstd::{prelude::*, ActorId};
use gtest::System;
mod utils;
use utils::*;

#[test]
fn fee_to() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    _ = set_fee_to_utils(&factory, FEE_SETTER, ActorId::from(NEW_FEE_TO));
    let res = fee_to_utils(&factory, FEE_SETTER);
    let message = FactoryEvent::FeeTo {
        address: ActorId::from(NEW_FEE_TO),
    }
    .encode();
    assert!(res.contains(&(FEE_SETTER, message)));
}

#[test]
fn set_fee_to() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    // should be sent by a fee_setter
    let res = set_fee_to_utils(&factory, FEE_SETTER, ActorId::from(NEW_FEE_TO));
    let message = FactoryEvent::FeeToSet {
        fee_to: ActorId::from(NEW_FEE_TO),
    }
    .encode();
    assert!(res.contains(&(FEE_SETTER, message)));

    // check if new fee_to is in state
    check_fee_to(&factory, ActorId::from(NEW_FEE_TO));
}

#[test]
fn set_fee_to_failures() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    // MUST fail since the USER is not a fee setter
    let res = set_fee_to_utils(&factory, USER, ActorId::from(NEW_FEE_TO));
    assert!(res.main_failed());
    // MUST fail since the NEW_FEE_TO a ZERO address
    let res = set_fee_to_utils(&factory, USER, ZERO_ID);
    assert!(res.main_failed());
}

#[test]
fn set_fee_to_setter() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    let res = set_fee_to_setter_utils(&factory, FEE_SETTER, ActorId::from(NEW_FEE_SETTER));
    let message = FactoryEvent::FeeToSetterSet {
        fee_to_setter: ActorId::from(NEW_FEE_SETTER),
    }
    .encode();
    assert!(res.contains(&(FEE_SETTER, message)));
    // check if new fee_to_setter is in state
    check_fee_to_setter(&factory, ActorId::from(NEW_FEE_SETTER));
}

#[test]
fn set_fee_to_setter_failures() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    // MUST fail since the USER is not a fee setter
    let res = set_fee_to_setter_utils(&factory, USER, ActorId::from(NEW_FEE_SETTER));
    assert!(res.main_failed());
    // MUST fail since the NEW_FEE_TO_SETTER a ZERO address
    let res = set_fee_to_setter_utils(&factory, FEE_SETTER, ZERO_ID);
    assert!(res.main_failed());
}

#[test]
fn create_pair() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    let token_a = ActorId::from(TOKEN_A);
    let token_b = ActorId::from(TOKEN_B);
    // MUST fail since token_a and token_b share the same address
    let res = create_pair_utils(&factory, USER, token_a, token_b);
    // There is no way to fully check against PairCreated
    // because of the pair_address being random
    // we should check for logs being non empty and not failed
    assert!(!res.main_failed());
    assert!(!res.log().is_empty());

    // check if the all pair length is equal to 1
    check_pair_len(&factory, 1);
}

#[test]
fn create_pair_failures() {
    let sys = System::new();
    init_factory(&sys);
    let factory = sys.get_program(1);
    let token_a = ActorId::from(TOKEN_A);
    let token_b = ActorId::from(TOKEN_B);
    // MUST fail since token_a and token_b share the same address
    let _ = create_pair_utils(&factory, USER, token_a, token_a);
    // MUST fail since token_a is a ZERO address
    let _ = create_pair_utils(&factory, USER, ZERO_ID, token_a);
    // MUST fail since the pair already exists
    let _ = create_pair_utils(&factory, USER, token_a, token_b);
    let _ = create_pair_utils(&factory, USER, token_a, token_b);
}
