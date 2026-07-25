//! Contract test for the checked-in instrument JSON examples.
//!
//! Every file under `tests/instruments/json_examples/` is a hand-maintained
//! reference payload that developers copy when authoring instrument JSON. None
//! of them (apart from `structured_credit_full.json`) was previously loaded by
//! any test, so they were free to rot silently as the wire format evolved — and
//! two of them had: `credit_default_swap.json` and `cds_index.json` still used
//! the pre-`rename_all` PascalCase `"IsdaNa"` for `CDSConvention` and predated
//! the `FocusedPricingOverrides` derive that flattens the three pricing-override
//! structs into a single `pricing_overrides` object.
//!
//! This test walks the directory and asserts every example deserializes under
//! the current schema, so drift fails loudly instead of misleading a reader.

use std::path::{Path, PathBuf};

use finstack_quant_valuations::instruments::json_loader::InstrumentEnvelope;

/// Absolute path to the `json_examples` directory.
fn json_examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/instruments/json_examples")
}

/// Every `.json` file in the examples directory, sorted for deterministic output.
fn example_files() -> Vec<PathBuf> {
    let dir = json_examples_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

/// Every checked-in instrument example must deserialize under the current schema.
///
/// Failures are collected so one run reports every stale fixture rather than
/// stopping at the first.
#[test]
fn every_json_example_deserializes_under_current_schema() {
    let files = example_files();
    assert!(
        !files.is_empty(),
        "no JSON examples found in {}",
        json_examples_dir().display()
    );

    let mut failures = Vec::new();
    for path in &files {
        if let Err(err) = InstrumentEnvelope::from_path(path) {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into(),
            );
            failures.push(format!("  {name}: {err}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} instrument JSON examples no longer deserialize under the \
         current schema. Regenerate them from the current Rust types rather \
         than hand-patching:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

/// Guards the directory itself: the walker must actually be finding files.
///
/// Without this, a rename or move of `json_examples/` would turn the contract
/// test above into a vacuous pass over an empty set.
#[test]
fn json_examples_directory_is_populated() {
    let files = example_files();
    assert!(
        files.len() >= 30,
        "expected the instrument JSON example corpus to contain at least 30 \
         files, found {} — did the directory move?",
        files.len()
    );
}
