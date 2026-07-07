//! Encoding-neutral vectors (p-vectors-equivalence).
//!
//! Vectors live under `vectors/` as JSON — fixtures plus expected verdicts —
//! shared with loom-spec's Python reference judges. The Rust side loads them
//! here; a verdict disagreement is INCOHERENCE, never a bug to fix one side.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// One vector: a fixture and the verdict every conformant judge must reach.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorTest {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub fixture: serde_json::Value,
    pub expected: serde_json::Value,
}

/// A vectors file: a described set of vectors for one artifact class or oracle.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorFile {
    #[serde(default)]
    pub description: String,
    pub vectors: Vec<VectorTest>,
}

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("I/O error reading {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("vector file {0} failed to parse: {1}")]
    Parse(PathBuf, serde_json::Error),
}

/// The crate's vectors directory.
pub fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors")
}

/// Load one vectors file by stem, e.g. `motion` → `vectors/motion.json`.
pub fn load_vector_file(stem: &str) -> Result<VectorFile, VectorError> {
    let path = vectors_dir().join(format!("{stem}.json"));
    let raw = std::fs::read_to_string(&path).map_err(|e| VectorError::Io(path.clone(), e))?;
    serde_json::from_str(&raw).map_err(|e| VectorError::Parse(path, e))
}

/// Load every vectors file in `vectors/`, sorted by file name.
pub fn load_all() -> Result<Vec<(String, VectorFile)>, VectorError> {
    let dir = vectors_dir();
    let mut stems: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| VectorError::Io(dir.clone(), e))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json")
                .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    stems.sort();
    stems
        .into_iter()
        .map(|stem| Ok((stem.clone(), load_vector_file(&stem)?)))
        .collect()
}
