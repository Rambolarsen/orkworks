#![allow(dead_code)]
//! Admission fence for the unqualified hierarchy storage profile (#1356).
//!
//! The incomplete catalog and generic journal are isolated test mechanics.
//! They cannot create, admit, reload or publish an accepted production store.

#[cfg(test)]
mod admission_tests;
pub(crate) mod connection;
#[cfg(test)]
mod mechanics;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HierarchyStorageError(pub &'static str);

type Result<T> = std::result::Result<T, HierarchyStorageError>;
