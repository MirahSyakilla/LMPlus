//! IL2CPP Collection types exports
//!
//! This module contains wrappers for standard IL2CPP collection types:
//! - `Il2cppArray`: Fixed-size arrays
//! - `Il2cppList`: Dynamic lists (generics)
//! - `Il2cppDictionary`: Key-value dictionaries (generics)
//! - `Il2cppString`: IL2CPP string implementation
pub mod array;
pub mod dictionary;
pub mod list;
pub mod string;

pub use array::Il2cppArray;
pub use dictionary::Il2cppDictionary;
pub use list::Il2cppList;
pub use string::Il2cppString;
