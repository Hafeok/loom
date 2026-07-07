//! Typed-row fetching (pat-typed-row-view).
//!
//! SPARQL SELECT solutions are mapped to JSON objects (one key per bound
//! variable) and deserialized into serde row structs. Queries fetch; verdicts
//! are judged by pure Rust functions elsewhere (p-queries-fetch-oracles-judge).

use oxigraph::model::Term;
use oxigraph::sparql::QueryResults;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::graph::NamedGraph;
use crate::sparql::SparqlBuilder;
use crate::store::LoomStore;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("query evaluation failed: {0}")]
    Evaluation(String),
    #[error("query did not return solutions")]
    NotSolutions,
    #[error("row deserialization failed: {0}")]
    Row(#[from] serde_json::Error),
}

/// Anything that can run a maintained query file against a named graph and
/// hand back typed rows.
pub trait QueryExecutor {
    /// Run `query` (a file under `queries/`, with a `$graph` placeholder)
    /// against `graph` and deserialize each solution into `T`.
    fn fetch_rows<T: DeserializeOwned>(
        &self,
        graph: &NamedGraph,
        query: &str,
    ) -> Result<Vec<T>, QueryError>;
}

impl QueryExecutor for LoomStore {
    fn fetch_rows<T: DeserializeOwned>(
        &self,
        graph: &NamedGraph,
        query: &str,
    ) -> Result<Vec<T>, QueryError> {
        let bound = SparqlBuilder::from_query_file(query)
            .bind_graph("graph", graph)
            .build();
        self.fetch_rows_bound(&bound)
    }
}

impl LoomStore {
    /// Run an already fully-bound query and deserialize the solutions.
    pub fn fetch_rows_bound<T: DeserializeOwned>(&self, query: &str) -> Result<Vec<T>, QueryError> {
        let results = oxigraph::sparql::SparqlEvaluator::new()
            .parse_query(query)
            .map_err(|e| QueryError::Evaluation(e.to_string()))?
            .on_store(self.store())
            .execute()
            .map_err(|e| QueryError::Evaluation(e.to_string()))?;
        let QueryResults::Solutions(solutions) = results else {
            return Err(QueryError::NotSolutions);
        };
        let mut rows = Vec::new();
        for solution in solutions {
            let solution = solution.map_err(|e| QueryError::Evaluation(e.to_string()))?;
            let mut obj = serde_json::Map::new();
            for (var, term) in solution.iter() {
                obj.insert(var.as_str().to_string(), term_to_json(term));
            }
            rows.push(serde_json::from_value(serde_json::Value::Object(obj))?);
        }
        Ok(rows)
    }
}

fn term_to_json(term: &Term) -> serde_json::Value {
    match term {
        Term::NamedNode(n) => serde_json::Value::String(n.as_str().to_string()),
        Term::BlankNode(b) => serde_json::Value::String(format!("_:{}", b.as_str())),
        Term::Literal(lit) => {
            let dt = lit.datatype().as_str();
            if dt == format!("{}integer", loom_ontology::XSD_NS)
                || dt == format!("{}long", loom_ontology::XSD_NS)
            {
                if let Ok(n) = lit.value().parse::<i64>() {
                    return serde_json::Value::Number(n.into());
                }
            }
            if dt == format!("{}boolean", loom_ontology::XSD_NS) {
                if let Ok(b) = lit.value().parse::<bool>() {
                    return serde_json::Value::Bool(b);
                }
            }
            serde_json::Value::String(lit.value().to_string())
        }
    }
}
