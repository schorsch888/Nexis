//! Nexis core library - identity, permissions, and domain types.
//!
//! This crate re-exports protocol types from `nexis-protocol` and provides
//! domain-specific extensions for the Nexis system.

pub mod context;
pub mod identity;
pub mod message;
pub mod permission;

pub use nexis_protocol::{Action, MemberId, MemberIdError, Message, MessageContent, Permissions};

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
