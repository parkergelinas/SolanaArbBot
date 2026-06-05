//! Storage and account state boundary.
//!
//! Canonical home for normalized Solana account views used by analysis stages.
//! The legacy [`accounts`] workspace crate re-exports this crate for backward
//! compatibility.

#![forbid(unsafe_code)]

pub mod cache;

pub use cache::AccountCache;
