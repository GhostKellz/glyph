pub mod client;
pub mod server;
pub mod protocol;
pub mod transport;
pub mod error;

pub use anyhow;
pub use serde_json as json;
pub use async_trait;

pub use error::{Error, Result};
pub use protocol::*;

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub use server::{Server, Tool, ToolCtx, ToolResult};