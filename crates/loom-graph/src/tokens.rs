//! e-token-graph — typed views over `loom:g/tokens` and the resolution oracle.
//!
//! The query fetches rows (pat-typed-row-view); the judge is a pure Rust
//! function over those facts (p-queries-fetch-oracles-judge): every component
//! token must resolve through the semantic tier to a primitive, with no cycle
//! and no dangling reference (loom-spec §2).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::graph::NamedGraph;
use crate::query::{QueryError, QueryExecutor};

/// Maintained query: every token with its tier, binding and owning component.
pub const TOKENS_QUERY: &str = include_str!("../queries/tokens.sparql");

/// One token row from the store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenRow {
    pub token: String,
    pub tier: String,
    #[serde(default)]
    pub binds: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
}

/// Fetch the token rows for a graph.
pub fn token_rows<T: QueryExecutor>(
    executor: &T,
    graph: &NamedGraph,
) -> Result<Vec<TokenRow>, QueryError> {
    executor.fetch_rows(graph, TOKENS_QUERY)
}

/// An incoherence in the token graph — fails validation (loom-spec §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TokenIncoherence {
    /// A binds edge points at a token that does not exist.
    Dangling { token: String, missing: String },
    /// The resolution path revisits a token.
    Cycle { tokens: Vec<String> },
    /// A component token never reaches a primitive through the semantic tier.
    NoSemanticPath { token: String },
}

/// The §7 token-resolution oracle: pure judge over queried rows.
pub fn judge_token_resolution(rows: &[TokenRow]) -> Vec<TokenIncoherence> {
    let by_id: HashMap<&str, &TokenRow> = rows.iter().map(|r| (r.token.as_str(), r)).collect();
    let mut incoherences = Vec::new();

    for row in rows {
        if let Some(target) = &row.binds {
            if !by_id.contains_key(target.as_str()) {
                incoherences.push(TokenIncoherence::Dangling {
                    token: row.token.clone(),
                    missing: target.clone(),
                });
            }
        }
    }

    // Walk each component token's resolution chain.
    for row in rows.iter().filter(|r| r.tier == "component") {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut path: Vec<String> = vec![row.token.clone()];
        let mut through_semantic = false;
        let mut current = row;
        seen.insert(row.token.as_str());
        loop {
            let Some(target) = &current.binds else {
                // Chain ends before a primitive.
                incoherences.push(TokenIncoherence::NoSemanticPath {
                    token: row.token.clone(),
                });
                break;
            };
            let Some(next) = by_id.get(target.as_str()) else {
                break; // already reported as dangling
            };
            if !seen.insert(next.token.as_str()) {
                path.push(next.token.clone());
                incoherences.push(TokenIncoherence::Cycle { tokens: path });
                break;
            }
            path.push(next.token.clone());
            match next.tier.as_str() {
                "semantic" => {
                    through_semantic = true;
                    current = next;
                }
                "primitive" => {
                    if !through_semantic {
                        incoherences.push(TokenIncoherence::NoSemanticPath {
                            token: row.token.clone(),
                        });
                    }
                    break;
                }
                _ => current = next,
            }
        }
    }

    // Cycles that only involve semantic tokens (no component entry point).
    for row in rows.iter().filter(|r| r.tier == "semantic") {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut path: Vec<String> = vec![row.token.clone()];
        let mut current = row;
        seen.insert(row.token.as_str());
        while let Some(target) = &current.binds {
            let Some(next) = by_id.get(target.as_str()) else {
                break;
            };
            if !seen.insert(next.token.as_str()) {
                if next.token == row.token {
                    path.push(next.token.clone());
                    // Report each semantic cycle once, from its smallest member.
                    if path[..path.len() - 1].iter().min() == Some(&row.token) {
                        incoherences.push(TokenIncoherence::Cycle { tokens: path });
                    }
                }
                break;
            }
            path.push(next.token.clone());
            current = next;
        }
    }

    incoherences
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(token: &str, tier: &str, binds: Option<&str>, component: Option<&str>) -> TokenRow {
        TokenRow {
            token: token.into(),
            tier: tier.into(),
            binds: binds.map(String::from),
            component: component.map(String::from),
        }
    }

    #[test]
    fn resolved_chain_is_coherent() {
        let rows = vec![
            row("t/button.bg", "component", Some("t/surface"), Some("Ref.Button")),
            row("t/surface", "semantic", Some("t/blue"), None),
            row("t/blue", "primitive", None, None),
        ];
        assert!(judge_token_resolution(&rows).is_empty());
    }

    #[test]
    fn dangling_reference_is_incoherent() {
        let rows = vec![row("t/button.bg", "component", Some("t/ghost"), Some("Ref.Button"))];
        let verdict = judge_token_resolution(&rows);
        assert!(verdict
            .iter()
            .any(|i| matches!(i, TokenIncoherence::Dangling { missing, .. } if missing == "t/ghost")));
    }

    #[test]
    fn cycle_is_incoherent() {
        let rows = vec![
            row("t/a", "semantic", Some("t/b"), None),
            row("t/b", "semantic", Some("t/a"), None),
        ];
        let verdict = judge_token_resolution(&rows);
        assert!(verdict.iter().any(|i| matches!(i, TokenIncoherence::Cycle { .. })));
    }

    #[test]
    fn component_binding_primitive_directly_skips_semantic_tier() {
        let rows = vec![
            row("t/button.bg", "component", Some("t/blue"), Some("Ref.Button")),
            row("t/blue", "primitive", None, None),
        ];
        let verdict = judge_token_resolution(&rows);
        assert!(verdict
            .iter()
            .any(|i| matches!(i, TokenIncoherence::NoSemanticPath { .. })));
    }
}
