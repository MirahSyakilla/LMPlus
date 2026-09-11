//! Named-pipe IPC protocol between LMPlus (host) and the injected agent.
//!
//! Wire format (single UTF-8 JSON line per request/response):
//!   request : {"action":"ping"} | {"action":"formation","index":0..=5}
//!           | {"action":"map3dview","mode":0..=2} | {"action":"zoom","value":<float>}
//!           | {"action":"switch_account"}
//!   response: {"ok":true,"detail":"..."} | {"ok":false,"error":"..."}


mod types;
pub use types::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(not(windows))]
mod host;
#[cfg(not(windows))]
pub use host::*;
