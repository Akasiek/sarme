#![allow(
    dead_code,
    reason = "scanner entry point will be called by the processing queue"
)]

mod discovery;
mod error;
mod file;
mod runner;
mod service;
pub(crate) mod summary;

#[cfg(test)]
mod tests;

pub(crate) use runner::Scanner;
