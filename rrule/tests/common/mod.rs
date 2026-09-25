//! Shared by the corpus runners: `corpus` reads and checks the data files,
//! `adapter` is the only part that calls the `rrule` crate.
pub mod adapter;
pub mod corpus;
