//! Module for taint analysis
pub mod taint_analysis;

use std::collections::{BTreeMap as Map, BTreeSet as Set};
use std::fmt::Display;

// Re-export analyze function
pub use taint_analysis::analyze;