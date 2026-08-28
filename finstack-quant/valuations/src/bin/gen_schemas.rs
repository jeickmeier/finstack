//! Generate every valuations JSON Schema and canonical instrument fixture.

use finstack_quant_core::schema::{
    deterministic_json_bytes, run_schema_generator, run_schema_index_generator, SchemaArtifact,
    SchemaGenerationCommand, SchemaGenerationMode,
};
use finstack_quant_core::{Error, Result};
use finstack_quant_valuations::instruments::json_loader::instrument_registry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

const FIXTURE_ROOT: &str = "tests/instruments/json_examples";

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
    let roots = ["schemas/common", "schemas/instruments", "schemas/results"];
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
    let schemas = finstack_quant_valuations::schema::artifacts();
    run_schema_index_generator(
        manifest_dir,
        Path::new("schemas/index.json"),
        &schemas,
        &command,
    )?;
    run_schema_registries(manifest_dir, &command, schemas)?;
    run_fixture_generator(manifest_dir, &command, fixtures)
}
