#![cfg_attr(not(feature = "std"), no_std)]

pub mod builtins;
pub mod completion;
pub mod editor;
pub mod highlight;
pub mod history;
pub mod parser;
pub mod platform;
pub mod prompt;
