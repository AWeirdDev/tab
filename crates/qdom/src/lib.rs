//! A `no_std` DOM library.
//!
//!

#![no_std]

extern crate alloc;

mod core;
pub mod prelude;

pub use crate::core::*;
