#![no_std]

#[cfg(not(feature = "binary-vendor"))]
mod state;

#[cfg(not(feature = "binary-vendor"))]
pub use state::*;

#[cfg(feature = "binary-vendor")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
