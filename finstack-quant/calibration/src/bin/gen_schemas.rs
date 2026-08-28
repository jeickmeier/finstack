//! Generate checked-in calibration and market-quote schemas.

use finstack_quant_core::schema::{
    run_schema_generator, run_schema_index_generator, SchemaGenerationCommand,
};
use std::path::Path;

fn main() -> finstack_quant_core::Result<()> {
    let command = SchemaGenerationCommand::from_env()?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts = finstack_quant_calibration::schema::artifacts();
    run_schema_index_generator(
        manifest_dir,
        Path::new("schemas/index.json"),
        &artifacts,
        &command,
    )?;
    let (calibration, market): (Vec<_>, Vec<_>) = artifacts
        .into_iter()
        .partition(|artifact| artifact.relative_path.starts_with("schemas/calibration/"));
    run_schema_generator(
        manifest_dir,
        Path::new("schemas/calibration"),
        &calibration,
        &command,
    )?;
    run_schema_generator(manifest_dir, Path::new("schemas/market"), &market, &command)
}
