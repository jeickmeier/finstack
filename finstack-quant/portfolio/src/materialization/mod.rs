//! Versioned, content-addressed portfolio materialization fast path.
//!
//! [`crate::portfolio::PortfolioSpec`] remains the portable embedded
//! interchange format. This module is the native bulk-loading path for
//! normalized stores: the outer document is parsed once, unique instrument
//! envelopes remain [`serde_json::value::RawValue`] until validation, and
//! positions share cached [`std::sync::Arc`] instrument instances.

mod cache;
mod envelope;
mod report;

use std::sync::Arc;

use crate::dependencies::DependencyIndex;
use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::position::Position;
use crate::types::PositionId;
use cache::{CacheKey, CachedArtifact};
use finstack_quant_core::canonical::CANONICAL_VERSION;
use finstack_quant_core::contract::{
    check_json_limits, ContractDescriptor, ContractError, Diagnostic, LoadLimits, LoadPhase,
    Severity, ValidationReport,
};
use finstack_quant_core::{HashMap, HashSet};
use finstack_quant_valuations::instruments::{InstrumentEnvelope, INSTRUMENT_CONTRACT};
use serde::Deserialize;
use serde_json::value::RawValue;

pub use cache::InstrumentArtifactCache;
pub use envelope::{
    InstrumentArtifact, MarketDependenciesSpec, MaterializedPosition, MaterializerInfo,
    PortfolioHeader, PortfolioMaterializationEnvelope,
};
pub use report::{MaterializationPhases, MaterializationReport};

/// Persistence contract for [`PortfolioMaterializationEnvelope`].
pub const PORTFOLIO_MATERIALIZATION_CONTRACT: ContractDescriptor =
    ContractDescriptor::new("finstack_quant.portfolio_materialization", 1);

struct PreparedArtifact {
    input_index: usize,
    artifact_id: String,
    content_hash: String,
    instrument_version: u32,
    envelope: serde_json::Value,
    encoded_bytes: usize,
    claimed_dependencies: Option<MarketDependenciesSpec>,
}

struct ValidatedInput {
    bundle: MaterializationInput,
    decoded: HashMap<String, CachedArtifact>,
    diagnostics: ValidationReport,
    parse_nanos: Option<u64>,
    validation_nanos: Option<u64>,
    decode_nanos: Option<u64>,
    cache_hits: usize,
    dependency_count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterializationInput {
    schema: String,
    portfolio: PortfolioHeader,
    instruments: Vec<ArtifactInput>,
    positions: Vec<MaterializedPosition>,
    #[serde(default, rename = "materializer")]
    _materializer: Option<MaterializerInfo>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactInput {
    artifact_id: String,
    #[serde(default)]
    content_hash: Option<String>,
    envelope: Box<RawValue>,
    #[serde(default)]
    dependencies: Option<Box<RawValue>>,
}

impl Portfolio {
    /// Materialize a portfolio from one strict, versioned bulk bundle.
    ///
    /// The outer bundle is checked by an allocation-free lexical depth scan and
    /// then parsed directly into its typed form once. Embedded instrument
    /// envelopes remain raw JSON until validation. Count and byte limits,
    /// contract versions, references, duplicate IDs, and claimed content hashes
    /// are checked before any runtime instrument is decoded. Each cache miss is
    /// decoded sequentially once, and all positions referencing it share the
    /// same [`Arc`].
    ///
    /// Use [`Self::from_spec`] for portable embedded interchange where each
    /// position carries its own optional instrument definition.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Complete UTF-8 JSON encoding of a
    ///   [`PortfolioMaterializationEnvelope`].
    /// * `cache` - Shared bounded cache for content-addressed runtime
    ///   instruments.
    /// * `limits` - Resource policy bounding bytes, artifacts, positions, and
    ///   retained diagnostics.
    ///
    /// # Returns
    ///
    /// A validated portfolio with rebuilt indices and a phase-instrumented
    /// materialization report.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ContractLimitExceeded`] when a configured resource
    /// limit is exceeded, [`Error::MaterializationFailed`] for malformed JSON,
    /// contract, hash, reference, dependency, decode, or final portfolio
    /// validation findings, and [`Error::Core`] when canonical hashing fails.
    pub fn from_materialization(
        bytes: &[u8],
        cache: &InstrumentArtifactCache,
        limits: &LoadLimits,
    ) -> crate::Result<(Portfolio, MaterializationReport)> {
        let ValidatedInput {
            bundle,
            decoded,
            mut diagnostics,
            parse_nanos,
            validation_nanos,
            decode_nanos,
            cache_hits,
            dependency_count,
        } = validate_and_decode(bytes, cache, limits)?;

        let build_started = report::start_timer();
        let book_by_position = position_book_index(&bundle, limits, &mut diagnostics);
        let mut positions = Vec::with_capacity(bundle.positions.len());
        for (index, materialized) in bundle.positions.iter().enumerate() {
            let Some(artifact) = decoded.get(&materialized.artifact_id) else {
                // Reference validation guarantees this cannot happen unless a
                // prior decode failed, which has already returned above.
                diagnostics.push_bounded(limits, missing_artifact_diagnostic(index, materialized));
                continue;
            };
            if artifact.instrument.id() != materialized.instrument_id {
                diagnostics.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/instrument-id-mismatch",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "position '{}' names instrument '{}' but artifact '{}' contains '{}'",
                            materialized.id,
                            materialized.instrument_id,
                            materialized.artifact_id,
                            artifact.instrument.id()
                        ),
                    )
                    .with_pointer(format!("/positions/{index}/instrument_id"))
                    .with_position_id(materialized.id.to_string())
                    .with_instrument_id(materialized.instrument_id.clone())
                    .with_revision_id(materialized.artifact_id.clone()),
                );
            }

            match Position::new(
                materialized.id.clone(),
                materialized.entity_id.clone(),
                materialized.instrument_id.clone(),
                Arc::clone(&artifact.instrument),
                materialized.quantity,
                materialized.unit,
            ) {
                Ok(mut position) => {
                    position.attributes.clone_from(&materialized.attributes);
                    position.meta.clone_from(&materialized.meta);
                    position.book_id = book_by_position.get(&materialized.id).cloned();
                    positions.push(position);
                }
                Err(error) => diagnostics.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/position-invalid",
                        LoadPhase::Build,
                        Severity::Error,
                        error.to_string(),
                    )
                    .with_pointer(format!("/positions/{index}"))
                    .with_position_id(materialized.id.to_string()),
                ),
            }
        }
        let build_nanos = report::elapsed_nanos(build_started);

        if diagnostics.has_errors() {
            return Err(Error::MaterializationFailed(Box::new(diagnostics)));
        }

        let index_started = report::start_timer();
        let mut portfolio = Portfolio {
            id: bundle.portfolio.id,
            name: bundle.portfolio.name,
            base_ccy: bundle.portfolio.base_ccy,
            as_of: bundle.portfolio.as_of,
            entities: bundle.portfolio.entities,
            positions,
            position_index: HashMap::default(),
            dependency_index: DependencyIndex::default(),
            entity_index: HashMap::default(),
            attribute_key_index: HashMap::default(),
            books: bundle.portfolio.books,
            tags: bundle.portfolio.tags,
            meta: bundle.portfolio.meta,
            evaluation_state_id: 0,
        };
        portfolio.rebuild_index();
        if let Err(error) = portfolio.validate() {
            diagnostics.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/invariants-invalid",
                    LoadPhase::Build,
                    Severity::Error,
                    error.to_string(),
                ),
            );
            return Err(Error::MaterializationFailed(Box::new(diagnostics)));
        }
        let index_nanos = report::elapsed_nanos(index_started);

        let timing_available = parse_nanos.is_some()
            && validation_nanos.is_some()
            && decode_nanos.is_some()
            && build_nanos.is_some()
            && index_nanos.is_some();
        let report = MaterializationReport {
            report: diagnostics,
            unique_instruments: bundle.instruments.len(),
            positions: bundle.positions.len(),
            dependencies: dependency_count,
            cache_hits,
            input_bytes: bytes.len(),
            timing_available,
            phase_nanos: MaterializationPhases {
                parse: parse_nanos.unwrap_or_default(),
                validate_versions: validation_nanos.unwrap_or_default(),
                decode_instruments: decode_nanos.unwrap_or_default(),
                build_positions: build_nanos.unwrap_or_default(),
                index_build: index_nanos.unwrap_or_default(),
            },
        };
        Ok((portfolio, report))
    }

    /// Strictly validate a materialization bundle without building a portfolio.
    ///
    /// This path parses the outer bundle, enforces resource and contract
    /// versions, verifies content hashes and references, resolves every
    /// artifact ID while decoding identical content only once, checks claimed
    /// dependencies and instrument semantics, and validates position values.
    /// It deliberately does not construct
    /// [`Position`] values, rebuild portfolio indices, or call
    /// [`Portfolio::validate`].
    ///
    /// # Arguments
    ///
    /// * `bytes` - Complete UTF-8 JSON materialization bundle.
    /// * `cache` - Shared bounded artifact cache used while decoding.
    /// * `limits` - Resource limits for bytes, counts, and diagnostics.
    ///
    /// # Returns
    ///
    /// Validation counters and timings. `build_positions` and `index_build`
    /// are always zero because those phases are outside this API boundary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ContractLimitExceeded`] for resource-limit failures,
    /// [`Error::MaterializationFailed`] for malformed or semantically invalid
    /// data, and [`Error::Core`] when canonical hashing fails.
    pub fn validate_materialization(
        bytes: &[u8],
        cache: &InstrumentArtifactCache,
        limits: &LoadLimits,
    ) -> crate::Result<MaterializationReport> {
        let ValidatedInput {
            bundle,
            decoded,
            mut diagnostics,
            parse_nanos,
            validation_nanos,
            decode_nanos,
            cache_hits,
            dependency_count,
        } = validate_and_decode(bytes, cache, limits)?;
        let semantic_started = report::start_timer();
        let _book_by_position = position_book_index(&bundle, limits, &mut diagnostics);
        for (index, position) in bundle.positions.iter().enumerate() {
            let Some(artifact) = decoded.get(&position.artifact_id) else {
                diagnostics.push_bounded(limits, missing_artifact_diagnostic(index, position));
                continue;
            };
            validate_position_semantics(index, position, artifact, limits, &mut diagnostics);
        }
        let semantic_nanos = report::elapsed_nanos(semantic_started);
        if diagnostics.has_errors() {
            return Err(Error::MaterializationFailed(Box::new(diagnostics)));
        }

        Ok(MaterializationReport {
            report: diagnostics,
            unique_instruments: bundle.instruments.len(),
            positions: bundle.positions.len(),
            dependencies: dependency_count,
            cache_hits,
            input_bytes: bytes.len(),
            timing_available: parse_nanos.is_some()
                && validation_nanos.is_some()
                && decode_nanos.is_some()
                && semantic_nanos.is_some(),
            phase_nanos: MaterializationPhases {
                parse: parse_nanos.unwrap_or_default(),
                validate_versions: validation_nanos
                    .unwrap_or_default()
                    .saturating_add(semantic_nanos.unwrap_or_default()),
                decode_instruments: decode_nanos.unwrap_or_default(),
                build_positions: 0,
                index_build: 0,
            },
        })
    }

    /// Export a runtime portfolio as a deduplicated materialization bundle.
    ///
    /// Instrument artifacts are emitted in first-position order and
    /// deduplicated by canonical envelope content hash. Instruments that do
    /// not implement
    /// [`finstack_quant_valuations::instruments::Instrument::to_instrument_json`]
    /// cause an error instead of producing a lossy `null` payload.
    ///
    /// Use [`Self::to_spec`] for the portable per-position interchange format.
    ///
    /// # Returns
    ///
    /// A strict current-version materialization envelope.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the runtime portfolio is invalid, an instrument
    /// cannot be serialized, canonical hashing or dependency extraction fails,
    /// or a strict raw instrument envelope cannot be encoded.
    pub fn to_materialization(&self) -> crate::Result<PortfolioMaterializationEnvelope> {
        self.validate()?;

        let mut instruments = Vec::new();
        let mut artifact_id_by_hash: HashMap<String, String> = HashMap::default();
        let mut positions = Vec::with_capacity(self.positions.len());
        let mut books = self.books.clone();

        for position in &self.positions {
            let instrument_json = position.instrument.to_instrument_json().ok_or_else(|| {
                Error::invalid_input(format!(
                    "instrument '{}' on position '{}' does not support materialization serialization",
                    position.instrument_id, position.position_id
                ))
            })?;
            let envelope = InstrumentEnvelope::new(instrument_json);
            let content_hash = envelope.content_hash().map_err(Error::Core)?;
            let artifact_id = if let Some(existing) = artifact_id_by_hash.get(&content_hash) {
                existing.clone()
            } else {
                let artifact_id = content_hash.clone();
                let dependencies = position
                    .instrument
                    .market_dependencies()
                    .map_err(Error::Core)?;
                let raw = serde_json::value::to_raw_value(&envelope).map_err(|error| {
                    Error::invalid_input(format!(
                        "failed to encode strict instrument envelope '{}': {error}",
                        position.instrument_id
                    ))
                })?;
                instruments.push(InstrumentArtifact {
                    artifact_id: artifact_id.clone(),
                    content_hash: Some(content_hash.clone()),
                    envelope: raw,
                    dependencies: Some(dependencies),
                });
                artifact_id_by_hash.insert(content_hash, artifact_id.clone());
                artifact_id
            };

            if let Some(book_id) = &position.book_id {
                let book = books.get_mut(book_id).ok_or_else(|| {
                    Error::validation(format!(
                        "Position '{}' references non-existent book '{}'",
                        position.position_id, book_id
                    ))
                })?;
                book.add_position(position.position_id.clone());
            }

            positions.push(MaterializedPosition {
                id: position.position_id.clone(),
                entity_id: position.entity_id.clone(),
                instrument_id: position.instrument_id.clone(),
                artifact_id,
                quantity: position.quantity,
                unit: position.unit,
                attributes: position.attributes.clone(),
                meta: position.meta.clone(),
            });
        }

        Ok(PortfolioMaterializationEnvelope {
            schema: PORTFOLIO_MATERIALIZATION_CONTRACT.schema_string(),
            portfolio: PortfolioHeader {
                id: self.id.clone(),
                name: self.name.clone(),
                base_ccy: self.base_ccy,
                as_of: self.as_of,
                entities: self.entities.clone(),
                books,
                tags: self.tags.clone(),
                meta: self.meta.clone(),
            },
            instruments,
            positions,
            materializer: None,
        })
    }
}

fn validate_and_decode(
    bytes: &[u8],
    cache: &InstrumentArtifactCache,
    limits: &LoadLimits,
) -> crate::Result<ValidatedInput> {
    let parse_started = report::start_timer();
    check_json_limits(bytes, limits).map_err(materialization_contract_error)?;
    let bundle: MaterializationInput = serde_json::from_slice(bytes).map_err(|error| {
        let mut report = ValidationReport::default();
        let diagnostic = match error.classify() {
            serde_json::error::Category::Data => Diagnostic::new(
                "contract/structure-invalid",
                LoadPhase::Structure,
                Severity::Error,
                error.to_string(),
            ),
            serde_json::error::Category::Io
            | serde_json::error::Category::Syntax
            | serde_json::error::Category::Eof => Diagnostic::from_parse_error(&error, None),
        };
        report.push_bounded(limits, diagnostic);
        Error::MaterializationFailed(Box::new(report))
    })?;
    let parse_nanos = report::elapsed_nanos(parse_started);

    let validation_started = report::start_timer();
    enforce_count_limit("artifacts", bundle.instruments.len(), limits.max_artifacts)?;
    enforce_count_limit("positions", bundle.positions.len(), limits.max_positions)?;
    let mut diagnostics = ValidationReport::default();
    if let Err(error) = PORTFOLIO_MATERIALIZATION_CONTRACT.parse_schema_strict(
        Some(&bundle.schema),
        "/schema",
        limits,
    ) {
        append_contract_error(error, &mut diagnostics, limits);
    }
    validate_references(&bundle, limits, &mut diagnostics);
    let prepared = prepare_artifacts(&bundle, limits, &mut diagnostics)?;
    if diagnostics.has_errors() {
        return Err(Error::MaterializationFailed(Box::new(diagnostics)));
    }
    let validation_nanos = report::elapsed_nanos(validation_started);

    let decode_started = report::start_timer();
    let mut decoded: HashMap<String, CachedArtifact> = HashMap::default();
    let mut cache_hits = 0usize;
    let mut dependency_count = 0usize;
    for artifact in prepared {
        let PreparedArtifact {
            input_index,
            artifact_id,
            content_hash,
            instrument_version,
            envelope,
            encoded_bytes,
            claimed_dependencies,
        } = artifact;
        let key = CacheKey::new(content_hash.clone(), instrument_version, CANONICAL_VERSION);
        let decode_result = cache.get_or_decode(key, encoded_bytes, || {
            let envelope: InstrumentEnvelope =
                serde_json::from_value(envelope).map_err(|error| {
                    Error::invalid_input(format!(
                        "invalid strict instrument envelope for artifact '{artifact_id}': {error}"
                    ))
                })?;
            let instrument: Arc<dyn finstack_quant_valuations::instruments::Instrument> =
                Arc::from(envelope.instrument.into_boxed().map_err(Error::Core)?);
            let dependencies = instrument.market_dependencies().map_err(Error::Core)?;
            Ok(CachedArtifact {
                instrument,
                dependencies,
            })
        });

        let (cached, hit) = match decode_result {
            Ok(result) => result,
            Err(error) => {
                diagnostics.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/instrument-invalid",
                        LoadPhase::Build,
                        Severity::Error,
                        error.to_string(),
                    )
                    .with_pointer(format!("/instruments/{input_index}/envelope"))
                    .with_revision_id(artifact_id),
                );
                continue;
            }
        };
        if hit {
            cache_hits += 1;
        }
        dependency_count = dependency_count
            .saturating_add(crate::dependencies::flatten_dependencies(&cached.dependencies).len());
        if let Some(claimed) = claimed_dependencies {
            if claimed != cached.dependencies {
                diagnostics.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/dependencies-mismatch",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "artifact '{artifact_id}' dependency claim does not match runtime extraction"
                        ),
                    )
                    .with_revision_id(artifact_id.clone())
                    .with_artifact_hash(content_hash),
                );
            }
        }
        decoded.insert(artifact_id, cached);
    }
    let decode_nanos = report::elapsed_nanos(decode_started);
    if diagnostics.has_errors() {
        return Err(Error::MaterializationFailed(Box::new(diagnostics)));
    }

    Ok(ValidatedInput {
        bundle,
        decoded,
        diagnostics,
        parse_nanos,
        validation_nanos,
        decode_nanos,
        cache_hits,
        dependency_count,
    })
}

fn validate_position_semantics(
    index: usize,
    position: &MaterializedPosition,
    artifact: &CachedArtifact,
    limits: &LoadLimits,
    diagnostics: &mut ValidationReport,
) {
    if artifact.instrument.id() != position.instrument_id {
        diagnostics.push_bounded(
            limits,
            Diagnostic::new(
                "portfolio/instrument-id-mismatch",
                LoadPhase::Semantic,
                Severity::Error,
                format!(
                    "position '{}' names instrument '{}' but artifact '{}' contains '{}'",
                    position.id,
                    position.instrument_id,
                    position.artifact_id,
                    artifact.instrument.id()
                ),
            )
            .with_pointer(format!("/positions/{index}/instrument_id"))
            .with_position_id(position.id.to_string())
            .with_instrument_id(position.instrument_id.clone())
            .with_revision_id(position.artifact_id.clone()),
        );
    }
    let invalid_quantity = !position.quantity.is_finite()
        || (matches!(position.unit, crate::position::PositionUnit::Percentage)
            && position.quantity.abs() > 100.0);
    if invalid_quantity {
        diagnostics.push_bounded(
            limits,
            Diagnostic::new(
                "portfolio/position-invalid",
                LoadPhase::Semantic,
                Severity::Error,
                format!(
                    "invalid quantity {} for position '{}'",
                    position.quantity, position.id
                ),
            )
            .with_pointer(format!("/positions/{index}/quantity"))
            .with_position_id(position.id.to_string()),
        );
    }
}

fn enforce_count_limit(what: &'static str, found: usize, limit: usize) -> crate::Result<()> {
    if found > limit {
        return Err(Error::contract_limit_exceeded(what, found, limit));
    }
    Ok(())
}

fn materialization_contract_error(error: ContractError) -> Error {
    match error {
        ContractError::LimitExceeded { what, found, limit } => {
            Error::contract_limit_exceeded(what, found, limit)
        }
        ContractError::Report(report) => Error::MaterializationFailed(report),
        ContractError::Core(error) => Error::Core(error),
        other => Error::invalid_input(other.to_string()),
    }
}

fn validate_references(
    bundle: &MaterializationInput,
    limits: &LoadLimits,
    report: &mut ValidationReport,
) {
    let mut artifacts: HashMap<&str, usize> = HashMap::default();
    for (index, artifact) in bundle.instruments.iter().enumerate() {
        if let Some(first) = artifacts.insert(&artifact.artifact_id, index) {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/duplicate-artifact-id",
                    LoadPhase::Semantic,
                    Severity::Error,
                    format!(
                        "duplicate artifact_id '{}' at indices {first} and {index}",
                        artifact.artifact_id
                    ),
                )
                .with_pointer(format!("/instruments/{index}/artifact_id"))
                .with_revision_id(artifact.artifact_id.clone()),
            );
        }
    }

    let mut positions: HashMap<&PositionId, usize> = HashMap::default();
    for (index, position) in bundle.positions.iter().enumerate() {
        if let Some(first) = positions.insert(&position.id, index) {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/duplicate-position-id",
                    LoadPhase::Semantic,
                    Severity::Error,
                    format!(
                        "duplicate position id '{}' at indices {first} and {index}",
                        position.id
                    ),
                )
                .with_pointer(format!("/positions/{index}/id"))
                .with_position_id(position.id.to_string()),
            );
        }
        if !artifacts.contains_key(position.artifact_id.as_str()) {
            report.push_bounded(limits, missing_artifact_diagnostic(index, position));
        }
    }
    validate_portfolio_envelope_invariants(bundle, &positions, limits, report);
}

fn validate_portfolio_envelope_invariants(
    bundle: &MaterializationInput,
    positions: &HashMap<&PositionId, usize>,
    limits: &LoadLimits,
    report: &mut ValidationReport,
) {
    for (entity_id, entity) in &bundle.portfolio.entities {
        if entity_id != &entity.id {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/entity-id-mismatch",
                    LoadPhase::Semantic,
                    Severity::Error,
                    format!(
                        "entity map key '{entity_id}' does not match embedded id '{}'",
                        entity.id
                    ),
                )
                .with_pointer(format!("/portfolio/entities/{entity_id}/id")),
            );
        }
    }
    for (index, position) in bundle.positions.iter().enumerate() {
        if !bundle.portfolio.entities.contains_key(&position.entity_id) {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/unknown-entity",
                    LoadPhase::Semantic,
                    Severity::Error,
                    format!(
                        "position '{}' references unknown entity '{}'",
                        position.id, position.entity_id
                    ),
                )
                .with_pointer(format!("/positions/{index}/entity_id"))
                .with_position_id(position.id.to_string()),
            );
        }
    }

    let mut position_books: HashMap<&PositionId, &crate::book::BookId> = HashMap::default();
    let mut child_parents: HashMap<&crate::book::BookId, &crate::book::BookId> = HashMap::default();
    for (book_id, book) in &bundle.portfolio.books {
        if book_id != &book.id {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/book-id-mismatch",
                    LoadPhase::Semantic,
                    Severity::Error,
                    format!(
                        "book map key '{book_id}' does not match embedded id '{}'",
                        book.id
                    ),
                )
                .with_pointer(format!("/portfolio/books/{book_id}/id")),
            );
        }
        if let Some(parent_id) = &book.parent_id {
            if !bundle.portfolio.books.contains_key(parent_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-missing-parent",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!("book '{book_id}' references missing parent '{parent_id}'"),
                    )
                    .with_pointer(format!("/portfolio/books/{book_id}/parent_id")),
                );
            }
        }
        for (position_index, position_id) in book.position_ids.iter().enumerate() {
            if !positions.contains_key(position_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-missing-position",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "book '{book_id}' references non-existent position '{position_id}'"
                        ),
                    )
                    .with_pointer(format!(
                        "/portfolio/books/{book_id}/position_ids/{position_index}"
                    ))
                    .with_position_id(position_id.to_string()),
                );
            }
            if let Some(first_book) = position_books.insert(position_id, book_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/position-multiple-books",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "position '{position_id}' is assigned to books '{first_book}' and '{book_id}'"
                        ),
                    )
                    .with_pointer(format!(
                        "/portfolio/books/{book_id}/position_ids/{position_index}"
                    ))
                    .with_position_id(position_id.to_string()),
                );
            }
        }
        for (child_index, child_id) in book.child_book_ids.iter().enumerate() {
            let pointer = format!("/portfolio/books/{book_id}/child_book_ids/{child_index}");
            let Some(child) = bundle.portfolio.books.get(child_id) else {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-missing-child",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!("book '{book_id}' references missing child '{child_id}'"),
                    )
                    .with_pointer(pointer),
                );
                continue;
            };
            if let Some(first_parent) = child_parents.insert(child_id, book_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-multiple-parents",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "book '{child_id}' is listed by parents '{first_parent}' and '{book_id}'"
                        ),
                    )
                    .with_pointer(pointer.clone()),
                );
            }
            if child.parent_id.as_ref() != Some(book_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-parent-child-mismatch",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!(
                            "book '{book_id}' lists '{child_id}' as child, but its parent is {:?}",
                            child.parent_id
                        ),
                    )
                    .with_pointer(pointer),
                );
            }
        }
    }

    for (book_id, book) in &bundle.portfolio.books {
        if let Some(parent_id) = &book.parent_id {
            if let Some(parent) = bundle.portfolio.books.get(parent_id) {
                if !parent.child_book_ids.contains(book_id) {
                    report.push_bounded(
                        limits,
                        Diagnostic::new(
                            "portfolio/book-parent-child-mismatch",
                            LoadPhase::Semantic,
                            Severity::Error,
                            format!(
                                "book '{book_id}' names parent '{parent_id}', but the parent does not list it as a child"
                            ),
                        )
                        .with_pointer(format!("/portfolio/books/{book_id}/parent_id")),
                    );
                }
            }
        }

        let mut visited = HashSet::default();
        visited.insert(book_id);
        let mut current = book.parent_id.as_ref();
        while let Some(parent_id) = current {
            if !visited.insert(parent_id) {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/book-cycle",
                        LoadPhase::Semantic,
                        Severity::Error,
                        format!("cycle detected in book hierarchy at '{parent_id}'"),
                    )
                    .with_pointer(format!("/portfolio/books/{book_id}/parent_id")),
                );
                break;
            }
            current = bundle
                .portfolio
                .books
                .get(parent_id)
                .and_then(|parent| parent.parent_id.as_ref());
        }
    }
}

fn missing_artifact_diagnostic(index: usize, position: &MaterializedPosition) -> Diagnostic {
    Diagnostic::new(
        "portfolio/missing-artifact",
        LoadPhase::Semantic,
        Severity::Error,
        format!(
            "position '{}' references missing artifact '{}'",
            position.id, position.artifact_id
        ),
    )
    .with_pointer(format!("/positions/{index}/artifact_id"))
    .with_position_id(position.id.to_string())
    .with_revision_id(position.artifact_id.clone())
}

fn prepare_artifacts(
    bundle: &MaterializationInput,
    limits: &LoadLimits,
    report: &mut ValidationReport,
) -> crate::Result<Vec<PreparedArtifact>> {
    let mut prepared = Vec::with_capacity(bundle.instruments.len());
    let mut hashes: HashMap<String, (usize, String)> = HashMap::default();

    for (index, artifact) in bundle.instruments.iter().enumerate() {
        let pointer = format!("/instruments/{index}/envelope");
        let claimed_dependencies = match &artifact.dependencies {
            Some(raw) => match envelope::deserialize_dependencies(raw) {
                Ok(dependencies) => Some(dependencies),
                Err(error) => {
                    report.push_bounded(
                        limits,
                        Diagnostic::new(
                            "portfolio/dependencies-invalid",
                            LoadPhase::Structure,
                            Severity::Error,
                            format!(
                                "artifact '{}' dependency claim is invalid: {error}",
                                artifact.artifact_id
                            ),
                        )
                        .with_pointer(format!("/instruments/{index}/dependencies"))
                        .with_revision_id(artifact.artifact_id.clone()),
                    );
                    None
                }
            },
            None => None,
        };
        let value: serde_json::Value = match serde_json::from_str(artifact.envelope.get()) {
            Ok(value) => value,
            Err(error) => {
                report.push_bounded(
                    limits,
                    Diagnostic::from_parse_error(&error, Some(pointer))
                        .with_revision_id(artifact.artifact_id.clone()),
                );
                continue;
            }
        };

        let mut structure_valid = true;
        let instrument_version = if let Some(object) = value.as_object() {
            if let Some(key) = object
                .keys()
                .find(|key| key.as_str() != "schema" && key.as_str() != "instrument")
            {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/instrument-envelope-invalid",
                        LoadPhase::Structure,
                        Severity::Error,
                        format!(
                            "artifact '{}' instrument envelope contains unknown field '{key}'",
                            artifact.artifact_id
                        ),
                    )
                    .with_pointer(format!("{pointer}/{key}"))
                    .with_revision_id(artifact.artifact_id.clone()),
                );
                structure_valid = false;
            }

            if !object.contains_key("instrument") {
                report.push_bounded(
                    limits,
                    Diagnostic::new(
                        "portfolio/instrument-envelope-required",
                        LoadPhase::Structure,
                        Severity::Error,
                        format!(
                            "artifact '{}' instrument envelope is missing 'instrument'",
                            artifact.artifact_id
                        ),
                    )
                    .with_pointer(format!("{pointer}/instrument"))
                    .with_revision_id(artifact.artifact_id.clone()),
                );
                structure_valid = false;
            }

            match object.get("schema") {
                Some(serde_json::Value::String(schema)) => {
                    match INSTRUMENT_CONTRACT.parse_schema_strict(
                        Some(schema),
                        &format!("{pointer}/schema"),
                        limits,
                    ) {
                        Ok(version) => Some(version),
                        Err(error) => {
                            append_contract_error(error, report, limits);
                            None
                        }
                    }
                }
                Some(_) => {
                    report.push_bounded(
                        limits,
                        Diagnostic::new(
                            "portfolio/instrument-envelope-required",
                            LoadPhase::Version,
                            Severity::Error,
                            format!(
                                "artifact '{}' requires a string instrument schema marker",
                                artifact.artifact_id
                            ),
                        )
                        .with_pointer(format!("{pointer}/schema"))
                        .with_revision_id(artifact.artifact_id.clone()),
                    );
                    None
                }
                None => {
                    report.push_bounded(
                        limits,
                        Diagnostic::new(
                            "portfolio/instrument-envelope-required",
                            LoadPhase::Version,
                            Severity::Error,
                            format!(
                                "artifact '{}' requires a strict instrument envelope with a schema marker",
                                artifact.artifact_id
                            ),
                        )
                        .with_pointer(format!("{pointer}/schema"))
                        .with_revision_id(artifact.artifact_id.clone()),
                    );
                    None
                }
            }
        } else {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/instrument-envelope-required",
                    LoadPhase::Version,
                    Severity::Error,
                    format!(
                        "artifact '{}' must contain a strict instrument envelope object",
                        artifact.artifact_id
                    ),
                )
                .with_pointer(pointer)
                .with_revision_id(artifact.artifact_id.clone()),
            );
            structure_valid = false;
            None
        };

        let content_hash =
            finstack_quant_core::canonical::content_hash(&value).map_err(Error::Core)?;
        let hash_valid = artifact.content_hash.as_ref().is_none_or(|claimed| {
            if claimed == &content_hash {
                return true;
            }
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/content-hash-mismatch",
                    LoadPhase::Hash,
                    Severity::Error,
                    format!(
                        "artifact '{}' claims hash '{}' but canonical envelope hash is '{}'",
                        artifact.artifact_id, claimed, content_hash
                    ),
                )
                .with_pointer(format!("/instruments/{index}/content_hash"))
                .with_revision_id(artifact.artifact_id.clone())
                .with_artifact_hash(content_hash.clone()),
            );
            false
        });

        if let Some((first_index, first_id)) =
            hashes.insert(content_hash.clone(), (index, artifact.artifact_id.clone()))
        {
            report.push_bounded(
                limits,
                Diagnostic::new(
                    "portfolio/duplicate-artifact-content",
                    LoadPhase::Semantic,
                    Severity::Warning,
                    format!(
                        "artifacts '{first_id}' and '{}' at indices {first_index} and {index} have identical content",
                        artifact.artifact_id
                    ),
                )
                .with_pointer(format!("/instruments/{index}/envelope"))
                .with_revision_id(artifact.artifact_id.clone())
                .with_artifact_hash(content_hash.clone()),
            );
        }

        if let Some(instrument_version) = instrument_version {
            if structure_valid && hash_valid {
                prepared.push(PreparedArtifact {
                    input_index: index,
                    artifact_id: artifact.artifact_id.clone(),
                    content_hash,
                    instrument_version,
                    envelope: value,
                    encoded_bytes: artifact.envelope.get().len(),
                    claimed_dependencies,
                });
            }
        }
    }

    Ok(prepared)
}

fn append_contract_error(error: ContractError, report: &mut ValidationReport, limits: &LoadLimits) {
    match error {
        ContractError::Report(source) => {
            for diagnostic in source.diagnostics {
                report.push_bounded(limits, diagnostic);
            }
            if source.truncated {
                report.truncated = true;
            }
        }
        other => report.push_bounded(
            limits,
            Diagnostic::new(
                "portfolio/instrument-contract-invalid",
                LoadPhase::Version,
                Severity::Error,
                other.to_string(),
            ),
        ),
    }
}

fn position_book_index(
    bundle: &MaterializationInput,
    _limits: &LoadLimits,
    _report: &mut ValidationReport,
) -> HashMap<PositionId, crate::book::BookId> {
    let mut by_position = HashMap::default();
    for (book_id, book) in &bundle.portfolio.books {
        for position_id in &book.position_ids {
            by_position.insert(position_id.clone(), book_id.clone());
        }
    }
    by_position
}
