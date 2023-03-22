#![no_std]

use dex_factory_io::*;
use gstd::{
    errors::Result as GstdResult, exec, msg, prelude::*, prog::ProgramGenerator, ActorId, CodeId,
    MessageId,
};
use hashbrown::HashMap;

#[cfg(feature = "binary-vendor")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

struct Contract {
    pair: CodeId,
    fee_to: ActorId,
    fee_to_setter: ActorId,
    pairs: HashMap<(ActorId, ActorId), ActorId>,
}

static mut STATE: Option<Contract> = None;

impl Contract {
    fn check_fee_to_setter(&self) -> Result<(), Error> {
        if self.fee_to_setter != msg::source() {
            Err(Error::AccessRestricted)
        } else {
            Ok(())
        }
    }

    fn set_fee_to_setter(&mut self, actor: ActorId) -> Result<Event, Error> {
        self.check_fee_to_setter()?;

        if actor == ActorId::zero() {
            return Err(Error::ZeroActorId);
        }

        self.fee_to_setter = actor;

        Ok(Event::FeeToSetterSet(actor))
    }

    fn set_fee_receiver(&mut self, actor: ActorId) -> Result<Event, Error> {
        self.check_fee_to_setter()?;

        self.fee_to = actor;

        Ok(Event::FeeToSet(actor))
    }

    async fn create_pair(&mut self, token_a: ActorId, token_b: ActorId) -> Result<Event, Error> {
        if token_a == token_b {
            return Err(Error::IdenticalTokens);
        }

        if token_a == ActorId::zero() || token_b == ActorId::zero() {
            return Err(Error::ZeroActorId);
        }

        let token_pair = if token_b > token_a {
            (token_b, token_a)
        } else {
            (token_a, token_b)
        };

        if self.pairs.contains_key(&token_pair) {
            return Err(Error::PairExist);
        }

        let (pair_actor, result): (_, Result<(), dex_pair_io::Error>) = ProgramGenerator::create_program_for_reply_as(
            self.pair,
            token_pair.encode(),
            0,
        )?.await?;

        result?;

        self.pairs.insert(token_pair, pair_actor);

        Ok(Event::PairCreated {
            token_pair,
            pair_actor,
            pair_number: self.pairs.len() as _,
        })
    }
}

#[no_mangle]
extern "C" fn init() {
    let result = process_init();
    let is_err = result.is_err();

    reply(result).expect("failed to encode or reply from `init()`");

    if is_err {
        exec::exit(ActorId::zero());
    }
}

fn process_init() -> Result<(), Error> {
    let Initialize {
        fee_to,
        pair,
        fee_to_setter,
    } = msg::load()?;

    unsafe {
        STATE = Some(Contract {
            pair,
            fee_to,
            fee_to_setter,
            pairs: HashMap::new(),
        });
    };

    Ok(())
}

#[gstd::async_main]
async fn main() {
    reply(process_handle().await).expect("failed to encode or reply `handle()`");
}

async fn process_handle() -> Result<Event, Error> {
    let action: Action = msg::load()?;
    let contract = state_mut();

    match action {
        Action::FeeToSetter(actor) => contract.set_fee_to_setter(actor),
        Action::FeeTo(actor) => contract.set_fee_receiver(actor),
        Action::CreatePair(token_a, token_b) => contract.create_pair(token_a, token_b).await,
        Action::GetFeeTo => Ok(Event::FeeToSet(contract.fee_to)),
    }
}

fn state_mut() -> &'static mut Contract {
    let state = unsafe { STATE.as_mut() };

    debug_assert!(state.is_some(), "state isn't initialized");

    unsafe { state.unwrap_unchecked() }
}

#[no_mangle]
extern "C" fn state() {
    let Contract {
        pair,
        fee_to,
        fee_to_setter: admin,
        pairs,
    } = state_mut();

    reply(State {
        pair: *pair,
        fee_to_setter: *admin,
        fee_to: *fee_to,
        pairs: pairs.into_iter().map(|(k, v)| (*k, *v)).collect(),
    })
    .expect("failed to encode or reply from `state()`");
}

#[no_mangle]
extern "C" fn metahash() {
    let metahash: [u8; 32] = include!("../.metahash");

    reply(metahash).expect("failed to encode or reply from `metahash()`");
}

fn reply(payload: impl Encode) -> GstdResult<MessageId> {
    msg::reply(payload, 0)
}
