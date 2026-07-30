//! Generate the shared deterministic materialization benchmark fixtures.

use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "../benches/materialization_fixtures.rs"]
mod materialization_fixtures;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/materialization-benchmarks"));
    fs::create_dir_all(&output_dir)?;
    for fixture in [
        materialization_fixtures::fixture_a(),
        materialization_fixtures::fixture_b(),
    ] {
        let path = output_dir.join(format!("{}.json", fixture.name));
        fs::write(&path, fixture.bytes)?;
        println!(
            "{} artifacts={} positions={} dependencies={}",
            path.display(),
            fixture.artifact_count,
            fixture.position_count,
            fixture.dependency_count
        );
    }
    println!("{}", output_dir.display());
    Ok(())
}
