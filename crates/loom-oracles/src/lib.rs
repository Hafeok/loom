//! loom-oracles — the §7 checks (pat-oracle-pair).
//!
//! Each oracle pairs a SPARQL fact-gatherer (a maintained query in loom-graph)
//! with a pure Rust judge. This crate re-exports the judges as the oracle
//! surface; it never touches oxigraph itself (s-store-single-owner).

pub use loom_graph::mode_contracts::{judge_contract_consistency, ConsistencyReport};
pub use loom_graph::motion::{
    judge_motion_references, judge_reduced_motion_fallback, MotionCheckReport, MotionReference,
};
pub use loom_graph::reification_table::{
    judge_leaf_completeness, judge_row_ambiguity, AmbiguousKey, CompletenessReport,
};
pub use loom_graph::tokens::{judge_token_resolution, TokenIncoherence};
