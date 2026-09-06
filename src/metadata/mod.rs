mod error;
mod model;
mod reader;

pub(crate) use model::{MetadataIssue, MetadataRead, TrackMetadata};
pub(crate) use reader::read;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
