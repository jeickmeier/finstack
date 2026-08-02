//! TypeScript generation for materialization contracts.

#![cfg(feature = "ts_export")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use finstack_quant_core::contract::{Diagnostic, LoadPhase, Severity, ValidationReport};
use finstack_quant_portfolio::{
    InstrumentArtifact, MaterializationPhases, MaterializationReport, MaterializedPosition,
    MaterializerInfo, PortfolioHeader, PortfolioMaterializationEnvelope,
    PortfolioMaterializationSchema,
};
use ts_rs::TS;

static CONFIG: OnceLock<ts_rs::Config> = OnceLock::new();
static OUTPUT_DIR: OnceLock<PathBuf> = OnceLock::new();

fn output_dir() -> &'static Path {
    OUTPUT_DIR
        .get_or_init(|| {
            std::env::var_os("FINSTACK_TS_OUTPUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../finstack-quant-wasm/types/generated")
                })
        })
        .as_path()
}

fn config() -> &'static ts_rs::Config {
    CONFIG.get_or_init(|| {
        fs::create_dir_all(output_dir()).expect("create generated types directory");
        ts_rs::Config::new().with_out_dir(output_dir())
    })
}

fn normalize_generated_types() {
    for entry in fs::read_dir(output_dir()).expect("read generated types directory") {
        let path = entry.expect("read generated type entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("ts") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("read generated type");
        let normalized = contents
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        if normalized != contents {
            fs::write(&path, normalized).expect("write normalized generated type");
        }
    }
}

#[test]
fn export_materialization_contract_types() {
    let config = config();

    LoadPhase::export(config).expect("export LoadPhase");
    Severity::export(config).expect("export Severity");
    Diagnostic::export(config).expect("export Diagnostic");
    ValidationReport::export(config).expect("export ValidationReport");
    MaterializationPhases::export(config).expect("export MaterializationPhases");
    MaterializationReport::export(config).expect("export MaterializationReport");
    MaterializerInfo::export(config).expect("export MaterializerInfo");
    PortfolioHeader::export(config).expect("export PortfolioHeader");
    PortfolioMaterializationSchema::export(config).expect("export PortfolioMaterializationSchema");
    InstrumentArtifact::export(config).expect("export InstrumentArtifact");
    MaterializedPosition::export(config).expect("export MaterializedPosition");
    PortfolioMaterializationEnvelope::export(config)
        .expect("export PortfolioMaterializationEnvelope");
    normalize_generated_types();
}
