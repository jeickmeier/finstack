//! Generate every valuations JSON Schema and canonical instrument fixture.

use finstack_quant_core::schema::{
    deterministic_json_bytes, run_schema_generator, SchemaArtifact, SchemaGenerationCommand,
    SchemaGenerationMode,
};
use finstack_quant_core::{Error, Result};
use finstack_quant_valuations::instruments::json_loader::instrument_registry;
use finstack_quant_valuations::instruments::{
    InstrumentEnvelope, InstrumentPricingOverrides, MetricPricingOverrides,
    ScenarioPricingOverrides,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

const FIXTURE_ROOT: &str = "tests/instruments/json_examples";

fn instrument_examples() -> Result<Vec<Value>> {
    let mut examples = Vec::new();
    for entry in instrument_registry() {
        let entry_examples = entry.examples().map_err(|error| {
            Error::Internal(format!(
                "build canonical {} instrument example: {error}",
                entry.tag
            ))
        })?;
        examples.extend(entry_examples);
    }
    Ok(examples)
}

fn schema_artifacts() -> Vec<SchemaArtifact> {
    let mut artifacts = vec![
        SchemaArtifact::new::<finstack_quant_core::types::Attributes>(
            "schemas/common/1/attributes.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "attributes.schema.json"
            ),
            "Attributes",
            "User-defined tags and key-value metadata for classification.",
        ),
        SchemaArtifact::new::<finstack_quant_core::contract::Diagnostic>(
            "schemas/common/1/diagnostic.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "diagnostic.schema.json"
            ),
            "Diagnostic",
            "One structured finding emitted while loading a persisted contract.",
        ),
        SchemaArtifact::new::<finstack_quant_core::contract::ValidationReport>(
            "schemas/common/1/validation_report.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "validation_report.schema.json"
            ),
            "Validation Report",
            "Bounded structured diagnostics emitted by persisted-contract validation.",
        ),
        SchemaArtifact::new::<finstack_quant_core::dates::BusinessDayConvention>(
            "schemas/common/1/business_day_convention.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "business_day_convention.schema.json"
            ),
            "Business Day Convention",
            "Business-day adjustment convention.",
        ),
        SchemaArtifact::new::<finstack_quant_core::currency::Currency>(
            "schemas/common/1/currency.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "currency.schema.json"
            ),
            "Currency",
            "ISO 4217 currency code.",
        ),
        SchemaArtifact::new::<finstack_quant_core::wire::DateWire>(
            "schemas/common/1/date.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "date.schema.json"
            ),
            "Date",
            "ISO 8601 calendar date string.",
        ),
        SchemaArtifact::new::<finstack_quant_core::dates::DayCount>(
            "schemas/common/1/day_count.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "day_count.schema.json"
            ),
            "Day Count",
            "Day-count convention.",
        ),
        SchemaArtifact::new::<finstack_quant_core::wire::DecimalWire>(
            "schemas/common/1/decimal.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "decimal.schema.json"
            ),
            "Decimal",
            "Exact decimal encoded as a JSON string.",
        ),
        SchemaArtifact::new::<finstack_quant_core::types::InstrumentId>(
            "schemas/common/1/id.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "id.schema.json"
            ),
            "Id",
            "Opaque string identifier.",
        ),
        SchemaArtifact::new::<finstack_quant_core::money::Money>(
            "schemas/common/1/money.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "money.schema.json"
            ),
            "Money",
            "Currency-tagged monetary amount.",
        ),
        SchemaArtifact::new::<InstrumentPricingOverrides>(
            "schemas/common/1/instrument_pricing_overrides.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "instrument_pricing_overrides.schema.json"
            ),
            "Instrument Pricing Overrides",
            "Instrument-owned market quote and model configuration overrides.",
        ),
        SchemaArtifact::new::<MetricPricingOverrides>(
            "schemas/common/1/metric_pricing_overrides.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "metric_pricing_overrides.schema.json"
            ),
            "Metric Pricing Overrides",
            "Metric-time pricing and finite-difference configuration.",
        ),
        SchemaArtifact::new::<ScenarioPricingOverrides>(
            "schemas/common/1/scenario_pricing_overrides.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "scenario_pricing_overrides.schema.json"
            ),
            "Scenario Pricing Overrides",
            "Scenario-only price and spread shocks.",
        ),
        SchemaArtifact::new::<finstack_quant_core::dates::Tenor>(
            "schemas/common/1/tenor.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/common/1/",
                "tenor.schema.json"
            ),
            "Tenor",
            "Parsed financial tenor.",
        ),
        SchemaArtifact::new::<InstrumentEnvelope>(
            "schemas/instruments/1/instrument.schema.json",
            concat!(
                "https://finstack_quant.dev/schemas/instrument/1/",
                "instrument.schema.json"
            ),
            "Finstack Quant Instrument",
            "Canonical v1 envelope for every supported financial instrument.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema)
        .with_examples(instrument_examples),
        SchemaArtifact::new::<
            finstack_quant_valuations::calibration::api::schema::CalibrationEnvelope,
        >(
            "schemas/calibration/1/calibration.schema.json",
            "https://finstack_quant.dev/schemas/calibration/1/calibration.schema.json",
            "Calibration",
            "Canonical typed calibration request and result envelope.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema),
        SchemaArtifact::new::<finstack_quant_valuations::market::quotes::market_quote::MarketQuote>(
            "schemas/market/1/market_quote.schema.json",
            "https://finstack_quant.dev/schemas/market/1/market_quote.schema.json",
            "Market Quote",
            "Canonical tagged market quote.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema),
        SchemaArtifact::new::<finstack_quant_valuations::results::ValuationResult>(
            "schemas/results/1/valuation_result.schema.json",
            "https://finstack_quant.dev/schemas/results/1/valuation_result.schema.json",
            "Valuation Result",
            "Canonical valuation result containing PV and typed metrics.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema),
    ];

    artifacts.extend(
        instrument_registry()
            .into_iter()
            .map(|entry| entry.artifact),
    );
    artifacts
}

fn fixture_artifacts() -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut expected = BTreeMap::new();
    for entry in instrument_registry() {
        let examples = entry.examples().map_err(|error| {
            Error::Internal(format!(
                "build canonical {} instrument fixture: {error}",
                entry.tag
            ))
        })?;
        let [example] = examples.as_slice() else {
            return Err(Error::Internal(format!(
                "{} must provide exactly one canonical fixture example",
                entry.tag
            )));
        };
        let path = PathBuf::from(entry.fixture_path);
        validate_relative_file(&path, Path::new(FIXTURE_ROOT))?;
        if expected
            .insert(path.clone(), deterministic_json_bytes(example)?)
            .is_some()
        {
            return Err(Error::Validation(format!(
                "duplicate instrument fixture path {}",
                path.display()
            )));
        }
    }
    Ok(expected)
}

fn validate_relative_file(path: &Path, root: &Path) -> Result<()> {
    if !path.starts_with(root)
        || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Validation(format!(
            "invalid generated fixture path {}",
            path.display()
        )));
    }
    Ok(())
}

fn actual_json_files(directory: &Path, base: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    if !directory.exists() {
        return Ok(files);
    }
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).map_err(|error| {
            Error::Internal(format!(
                "read generated directory {}: {error}",
                current.display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                Error::Internal(format!("read generated directory entry: {error}"))
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                Error::Internal(format!(
                    "inspect generated path {}: {error}",
                    path.display()
                ))
            })?;
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("json")
            {
                files.insert(
                    path.strip_prefix(base)
                        .map_err(|error| {
                            Error::Internal(format!(
                                "generated path {} is outside {}: {error}",
                                path.display(),
                                base.display()
                            ))
                        })?
                        .to_path_buf(),
                );
            }
        }
    }
    Ok(files)
}

fn remove_empty_directories(directory: &Path, owned_root: &Path) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(directory).map_err(|error| {
        Error::Internal(format!("read directory {}: {error}", directory.display()))
    })? {
        let path = entry
            .map_err(|error| Error::Internal(format!("read directory entry: {error}")))?
            .path();
        if path.is_dir() {
            remove_empty_directories(&path, owned_root)?;
        }
    }
    if directory != owned_root
        && std::fs::read_dir(directory)
            .map_err(|error| {
                Error::Internal(format!("read directory {}: {error}", directory.display()))
            })?
            .next()
            .is_none()
    {
        std::fs::remove_dir(directory).map_err(|error| {
            Error::Internal(format!(
                "remove empty directory {}: {error}",
                directory.display()
            ))
        })?;
    }
    Ok(())
}

fn run_fixture_generator(
    manifest_dir: &Path,
    command: &SchemaGenerationCommand,
    expected: BTreeMap<PathBuf, Vec<u8>>,
) -> Result<()> {
    if command.mode == SchemaGenerationMode::List {
        for path in expected.keys() {
            println!("{}", path.display());
        }
        return Ok(());
    }

    let base = command.output_root.as_deref().unwrap_or(manifest_dir);
    let owned_root = base.join(FIXTURE_ROOT);
    let actual = actual_json_files(&owned_root, base)?;
    let expected_paths = expected.keys().cloned().collect::<BTreeSet<_>>();
    let extra = actual
        .difference(&expected_paths)
        .cloned()
        .collect::<Vec<_>>();

    match command.mode {
        SchemaGenerationMode::Check => {
            let mut drift = Vec::new();
            for (relative_path, bytes) in &expected {
                match std::fs::read(base.join(relative_path)) {
                    Ok(actual) if actual == *bytes => {}
                    Ok(_) => drift.push(format!("changed {}", relative_path.display())),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        drift.push(format!("missing {}", relative_path.display()));
                    }
                    Err(error) => {
                        return Err(Error::Internal(format!(
                            "read fixture {}: {error}",
                            relative_path.display()
                        )))
                    }
                }
            }
            drift.extend(extra.iter().map(|path| format!("extra {}", path.display())));
            if drift.is_empty() {
                Ok(())
            } else {
                Err(Error::Validation(format!(
                    "instrument fixtures are not current:\n{}",
                    drift.join("\n")
                )))
            }
        }
        SchemaGenerationMode::Write => {
            for (relative_path, bytes) in expected {
                let path = base.join(&relative_path);
                if std::fs::read(&path).ok().as_deref() == Some(bytes.as_slice()) {
                    continue;
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        Error::Internal(format!(
                            "create fixture directory {}: {error}",
                            parent.display()
                        ))
                    })?;
                }
                std::fs::write(&path, bytes).map_err(|error| {
                    Error::Internal(format!("write fixture {}: {error}", path.display()))
                })?;
                println!("updated {}", relative_path.display());
            }
            for relative_path in extra {
                let path = base.join(&relative_path);
                std::fs::remove_file(&path).map_err(|error| {
                    Error::Internal(format!("remove stale fixture {}: {error}", path.display()))
                })?;
                println!("removed stale {}", relative_path.display());
            }
            remove_empty_directories(&owned_root, &owned_root)
        }
        SchemaGenerationMode::List => Ok(()),
    }
}

fn run_schema_registries(
    manifest_dir: &Path,
    command: &SchemaGenerationCommand,
    artifacts: Vec<SchemaArtifact>,
) -> Result<()> {
    let roots = [
        "schemas/calibration",
        "schemas/common",
        "schemas/instruments",
        "schemas/market",
        "schemas/results",
    ];
    let mut registries = roots
        .into_iter()
        .map(|root| (root, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for artifact in artifacts {
        let root = roots
            .into_iter()
            .find(|root| Path::new(artifact.relative_path).starts_with(root))
            .ok_or_else(|| {
                Error::Validation(format!(
                    "schema artifact is outside a registered valuations root: {}",
                    artifact.relative_path
                ))
            })?;
        registries
            .get_mut(root)
            .ok_or_else(|| Error::Internal(format!("missing schema registry for {root}")))?
            .push(artifact);
    }
    for (root, artifacts) in registries {
        run_schema_generator(manifest_dir, Path::new(root), &artifacts, command)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let command = SchemaGenerationCommand::from_env()?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = fixture_artifacts()?;
    let schemas = schema_artifacts();
    run_schema_registries(manifest_dir, &command, schemas)?;
    run_fixture_generator(manifest_dir, &command, fixtures)
}
