#![no_std]

use factory_io::*;
use gstd::{msg, prelude::*, ActorId};

const ZERO_ID: ActorId = ActorId::zero();

#[derive(Debug, Default)]
pub struct Factory {
    pub owner_id: ActorId,
    pub fee_to: ActorId,
    pub fee_to_setter: ActorId,
    pub all_pairs: Vec<ActorId>,
    pub pairs: BTreeMap<(ActorId, ActorId), ActorId>,
}

static mut FACTORY: Option<Factory> = None;

impl Factory {
    /// Sets a fee_to address
    /// `_fee_to` MUST be a non-zero address
    /// Message source MUST be a fee_to_setter of the contract
    /// Arguments:
    /// * `_fee_to` is a new fee_to address
    fn set_fee_to(&mut self, _fee_to: ActorId) {
        if self.fee_to_setter != msg::source() {
            panic!("FACTORY: Setting fee_to is forbidden for this address");
        }
        if _fee_to == ZERO_ID {
            panic!("FACTORY: Fee_to can not be a ZERO address");
        }
        self.fee_to = _fee_to;

        msg::reply(FactoryEvent::FeeToSet { fee_to: _fee_to }, 0)
            .expect("FACTORY: Error during a replying with FactoryEvent::FeeToSet");
    }

    /// Sets a fee_to_setter address
    /// `_fee_to_setter` MUST be a non-zero address
    /// Message source MUST be a fee_to_setter of the contract
    /// Arguments:
    /// * `_fee_to_setter` is a new fee_to_setter address
    fn set_fee_to_setter(&mut self, _fee_to_setter: ActorId) {
        if self.fee_to_setter != msg::source() {
            panic!("FACTORY: Changing fee_to_setter is forbidden for this address");
        }
        if _fee_to_setter == ZERO_ID {
            panic!("FACTORY: Fee_to_setter can not be a ZERO address");
        }
        self.fee_to_setter = _fee_to_setter;

        msg::reply(
            FactoryEvent::FeeToSetterSet {
                fee_to_setter: _fee_to_setter,
            },
            0,
        )
        .expect("FACTORY: Error during a replying with FactoryEvent::FeeToSetterSet");
    }

    /// Creates and deploys a new pair
    /// Both token address MUST be different and non-zero
    /// Also the pair MUST not be created already
    /// Arguments:
    /// * `token_a` is the first token address
    /// * `token_b` is the second token address
    async fn create_pair(&mut self, mut token_a: ActorId, mut token_b: ActorId) {
        (token_a, token_b) = if token_a > token_b {
            (token_b, token_a)
        } else {
            (token_a, token_b)
        };
        if token_a == token_b {
            panic!("FACTORY: Identical token addresses");
        }
        if token_a == ZERO_ID || token_b == ZERO_ID {
            panic!("FACTORY: One of your addresses is a ZERO one");
        }
        if self.pairs.contains_key(&(token_a, token_b)) {
            panic!("FACTORY: Such pair already exists.");
        }

        // create program
        let program_id = ActorId::zero();
        self.pairs
            .entry((token_a, token_b))
            .or_insert(program_id);

        self.all_pairs.push(program_id);
        msg::reply(
            FactoryEvent::PairCreated {
                token_a,
                token_b,
                pair_address: ZERO_ID,
                pairs_length: 10,
            },
            0,
        )
        .expect("FACTORY: Error during a replying with FactoryEvent::CreatePair");
    }
}

#[no_mangle]
extern "C" fn init() {
    let config: InitFactory = msg::load().expect("Unable to decode InitEscrow");
    let factory = Factory {
        fee_to_setter: config.fee_to_setter,
        owner_id: msg::source(),
        ..Default::default()
    };
    unsafe {
        FACTORY = Some(factory);
    }
}

#[gstd::async_main]
async unsafe fn main() {
    let action: FactoryAction = msg::load().expect("Unable to decode FactoryAction");
    let factory = unsafe { FACTORY.get_or_insert(Default::default()) };
    match action {
        FactoryAction::SetFeeTo { fee_to } => {
            factory.set_fee_to(fee_to);
        }
        FactoryAction::SetFeeToSetter { fee_to_setter } => {
            factory.set_fee_to_setter(fee_to_setter);
        }
        FactoryAction::CreatePair { token_a, token_b } => {
            factory.create_pair(token_a, token_b).await;
        }
        FactoryAction::FeeTo => {
            msg::reply(FactoryEvent::FeeTo { address: factory.fee_to }, 0)
            .expect("FACTORY: Error during a replying with FactoryEvent::FeeTo");
        }
    }
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let state: FactoryStateQuery = msg::load().expect("Unable to decode FactoryStateQuey");
    let factory = unsafe { FACTORY.get_or_insert(Default::default()) };
    let reply = match state {
        FactoryStateQuery::FeeTo => FactoryStateReply::FeeTo {
            address: factory.fee_to,
        },
        FactoryStateQuery::FeeToSetter => FactoryStateReply::FeeToSetter {
            address: factory.fee_to_setter,
        },
        FactoryStateQuery::PairAddress { token_a, token_b } => {
            let (t1, t2) = if token_a > token_b {
                (token_b, token_a)
            } else {
                (token_a, token_b)
            };
            FactoryStateReply::PairAddress {
                address: *factory
                    .pairs
                    .get(&(t1, t2))
                    .expect("No such token pair"),
            }
        },
        FactoryStateQuery::AllPairsLength => FactoryStateReply::AllPairsLength {
            length: factory.all_pairs.len() as u32,
        },
        FactoryStateQuery::Owner => FactoryStateReply::Owner {
            address: factory.owner_id,
        },
    };
    gstd::util::to_leak_ptr(reply.encode())
}

gstd::metadata! {
    title: "DEXFactory",
    init:
        input: InitFactory,
    handle:
        input: FactoryAction,
        output: FactoryEvent,
    state:
        input: FactoryStateQuery,
        output: FactoryStateReply,
}
