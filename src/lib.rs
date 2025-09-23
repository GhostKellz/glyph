//! Glyph MCP Library
//! Enterprise-grade Rust library for Model Context Protocol (MCP)
//! Integrated with Rune (Zig) for high-performance tool execution

pub mod error;
pub mod rune_ffi;

// Re-export for convenience
pub use error::*;