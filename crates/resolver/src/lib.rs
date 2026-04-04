//! Name resolution for `ocelot`.

pub(crate) mod declaration_indexer;
pub(crate) mod diagnostics;
pub(crate) mod effect_propagation;
pub mod resolution;
pub(crate) mod resolver;

#[cfg(test)]
mod tests;
