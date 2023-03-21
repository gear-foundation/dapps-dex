#![no_std]

use dex_factory_io::*;
use gmeta::{metawasm, Metadata};
use gstd::{prelude::*, ActorId};

#[cfg(feature = "binary-vendor")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[metawasm]
pub mod metafns {
    pub type State = <ContractMetadata as Metadata>::State;

    pub fn fee_to(state: State) -> ActorId {
        state.fee_to
    }

    pub fn fee_to_setter(state: State) -> ActorId {
        state.fee_to_setter
    }

    pub fn pair(state: State, mut pair: (ActorId, ActorId)) -> ActorId {
        if pair.1 > pair.0 {
            pair = (pair.1, pair.0);
        }

        state
            .pairs
            .into_iter()
            .find_map(|(existing_pair, actor)| (existing_pair == pair).then_some(actor))
            .unwrap_or_default()
    }

    pub fn all_pairs_length(state: State) -> u32 {
        state.pairs.len() as _
    }

    pub fn all_pairs(state: State) -> Vec<((ActorId, ActorId), ActorId)> {
        state.pairs
    }
}
