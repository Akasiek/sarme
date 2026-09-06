#![allow(
    dead_code,
    reason = "LRCLIB client will be called by the processing queue"
)]

mod client;
mod error;
mod model;

pub(crate) use client::LrclibClient;
pub(crate) use error::LrclibError;

#[cfg(test)]
mod tests;
