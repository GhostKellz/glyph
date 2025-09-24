//! Glyph MCP Library
//! Enterprise-grade Rust library for Model Context Protocol (MCP)
//! Ready for high-performance language integrations via FFI

pub mod client;
pub mod error;
pub mod ffi;
pub mod protocol;
pub mod server;
pub mod transport;

// Re-export for convenience
pub use error::*;