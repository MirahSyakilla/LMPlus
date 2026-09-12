//! Debugging and analysis tools for IL2CPP
//!
//! This module contains tools for dumping C# pseudo-code from IL2CPP applications.
pub mod cs;

pub use cs::{dump, dump_all, dump_all_to, dump_assembly, dump_to};
