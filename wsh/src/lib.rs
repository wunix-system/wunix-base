#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]

pub mod builtins;
pub mod completion;
pub mod editor;
pub mod highlight;
pub mod history;
pub mod parser;
pub mod platform;
pub mod prompt;
pub mod runner;
