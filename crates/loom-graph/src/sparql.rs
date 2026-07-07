//! Parameter binding for the maintained queries under `queries/`.
//!
//! Queries live in files (s-queries-maintained); the builder only substitutes
//! `$name` placeholders — it never assembles SPARQL text from scratch.

use crate::graph::NamedGraph;

/// Binds `$name` placeholders in a maintained query file.
#[derive(Debug, Clone)]
pub struct SparqlBuilder {
    query: String,
}

impl SparqlBuilder {
    /// Start from the contents of a file under `queries/`.
    pub fn from_query_file(query: &str) -> Self {
        Self {
            query: query.to_string(),
        }
    }

    /// Bind `$name` to an IRI (emitted as `<iri>`).
    pub fn bind_iri(mut self, name: &str, iri: &str) -> Self {
        self.query = self
            .query
            .replace(&format!("${name}"), &format!("<{iri}>"));
        self
    }

    /// Bind `$name` to the IRI of a named graph.
    pub fn bind_graph(self, name: &str, graph: &NamedGraph) -> Self {
        let iri = graph.iri();
        self.bind_iri(name, &iri)
    }

    /// Bind `$name` to a string literal (quoted and escaped).
    pub fn bind_str(mut self, name: &str, value: &str) -> Self {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        self.query = self
            .query
            .replace(&format!("${name}"), &format!("\"{escaped}\""));
        self
    }

    pub fn build(self) -> String {
        self.query
    }
}
