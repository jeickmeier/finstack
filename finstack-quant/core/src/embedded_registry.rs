//! Shared loader for embedded JSON registries with optional config overrides.
//!
//! Registries keep ownership of their domain payload and validation logic. This
//! loader handles JSON decoding, process-wide caching for the embedded payload,
//! and optional lookup through [`FinstackConfig`](crate::config::FinstackConfig)
//! extensions.
//!
//! Callers:
//! - Declare a `static MY_REGISTRY: EmbeddedJsonRegistry<MyRegistry> = …;`
//!   (or `EmbeddedJsonRegistry<MyRegistry, MyFile>` when the on-disk document
//!   shape differs from the resolved registry type).
//! - Provide a `fn build(doc: MyFile) -> Result<MyRegistry>` that validates and
//!   converts the decoded document.
//! - Call `MY_REGISTRY.load(build)` and, when an extension key was declared,
//!   `MY_REGISTRY.load_from_config(cfg, build)`.
//!
//! # Examples
//! ```rust
//! use finstack_quant_core::embedded_registry::EmbeddedJsonRegistry;
//! use finstack_quant_core::{Error, Result};
//! use serde::Deserialize;
//!
//! #[derive(Clone, Deserialize)]
//! struct Registry {
//!     version: u32,
//! }
//!
//! static REGISTRY: EmbeddedJsonRegistry<Registry> =
//!     EmbeddedJsonRegistry::new(r#"{"version": 1}"#, None, "example");
//!
//! fn validate(registry: Registry) -> Result<Registry> {
//!     if registry.version == 1 {
//!         Ok(registry)
//!     } else {
//!         Err(Error::Validation("unsupported version".to_string()))
//!     }
//! }
//!
//! let registry = REGISTRY.load(validate)?;
//! assert_eq!(registry.version, 1);
//! # Ok::<(), Error>(())
//! ```

use crate::config::FinstackConfig;
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::OnceLock;

/// Loader for a single versioned JSON registry shipped as a compile-time asset.
///
/// `T` is the resolved registry type handed to callers; `D` is the serde
/// document type decoded from the JSON text (defaults to `T`). The loader
/// caches the built registry in a `OnceLock` so the JSON parse + validation
/// cost is paid at most once per process. A failed first load is cached too and
/// returned by clone on every subsequent call, which keeps the outcome
/// deterministic for compile-time assets.
pub struct EmbeddedJsonRegistry<T: 'static, D = T> {
    /// Raw JSON content, typically `include_str!("…json")`.
    embedded_raw: &'static str,
    /// Configuration extension key used by `load_from_config` to look for a
    /// replacement document before falling back to the embedded copy.
    extension_key: Option<&'static str>,
    /// Human-readable label used in error messages (e.g. "credit assumptions").
    parse_label: &'static str,
    /// Process-wide cache of the parsed-and-validated embedded registry.
    cell: OnceLock<Result<T>>,
    /// Marker tying the loader to its serde document type.
    document: PhantomData<fn() -> D>,
}

impl<T, D> EmbeddedJsonRegistry<T, D>
where
    T: Clone + Send + Sync + 'static,
    D: DeserializeOwned,
{
    /// Construct a loader. Intended for `static` storage.
    ///
    /// # Arguments
    ///
    /// * `embedded_raw` - Compile-time JSON text of the registry, usually an
    ///   `include_str!` of the versioned data file; decoded as `D` on first load.
    /// * `extension_key` - Optional [`FinstackConfig`] extension key (for example
    ///   `"core.rating_scales.v1"`) whose value, when present, replaces the
    ///   embedded document in [`load_from_config`](Self::load_from_config).
    ///   `None` means the registry has no configuration override path.
    /// * `parse_label` - Short human-readable registry name spliced into parse
    ///   and validation error messages (for example `"rating-scale"`).
    pub const fn new(
        embedded_raw: &'static str,
        extension_key: Option<&'static str>,
        parse_label: &'static str,
    ) -> Self {
        Self {
            embedded_raw,
            extension_key,
            parse_label,
            cell: OnceLock::new(),
            document: PhantomData,
        }
    }

    /// Load (and cache) the embedded registry, applying `build`.
    ///
    /// Returns a borrowed reference to the cached value. If parsing or
    /// `build` fails, the failure is also cached and returned by clone on
    /// every subsequent call.
    ///
    /// # Arguments
    ///
    /// * `build` - Validation/conversion hook invoked once with the decoded
    ///   document; it must reject invalid content with [`Error::Validation`]
    ///   and return the resolved registry otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if the embedded JSON does not decode as
    /// `D`, or propagates the error returned by `build`.
    pub fn load<F>(&self, build: F) -> Result<&T>
    where
        F: FnOnce(D) -> Result<T>,
    {
        match self
            .cell
            .get_or_init(|| parse_and_build(self.embedded_raw, self.parse_label, build))
        {
            Ok(registry) => Ok(registry),
            Err(err) => Err(err.clone()),
        }
    }

    /// Load from configuration, preferring an extension override over the
    /// embedded copy. The same `build` hook is applied to both paths, and the
    /// returned registry is owned so callers can amend their configured view
    /// without mutating the cached embedded default.
    ///
    /// # Arguments
    ///
    /// * `config` - Library configuration whose `extensions` map is consulted
    ///   under the loader's extension key; when the key is absent (or the
    ///   loader declared none) a clone of the embedded registry is returned.
    /// * `build` - Validation/conversion hook applied to the decoded override
    ///   document or, on fallback, to the embedded document (see [`load`](Self::load)).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if the configured extension exists but does
    /// not decode as `D` or is rejected by `build`, or if the embedded fallback
    /// fails to load. Invalid configured data never silently falls back to the
    /// embedded copy.
    pub fn load_from_config<F>(&self, config: &FinstackConfig, build: F) -> Result<T>
    where
        F: Fn(D) -> Result<T>,
    {
        let override_value = self
            .extension_key
            .and_then(|key| config.extensions.get(key));
        if let Some(value) = override_value {
            let document = serde_json::from_value::<D>(value.clone()).map_err(|err| {
                Error::Validation(format!(
                    "failed to parse {} registry extension: {err}",
                    self.parse_label
                ))
            })?;
            build(document)
        } else {
            Ok(self.load(build)?.clone())
        }
    }
}

fn parse_and_build<T, D, F>(raw: &str, parse_label: &str, build: F) -> Result<T>
where
    D: DeserializeOwned,
    F: FnOnce(D) -> Result<T>,
{
    let document = serde_json::from_str::<D>(raw).map_err(|err| {
        Error::Validation(format!(
            "failed to parse embedded {parse_label} registry: {err}"
        ))
    })?;
    build(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
    struct DummyRegistry {
        version: u32,
    }

    const RAW: &str = r#"{"version": 1}"#;

    static REG: EmbeddedJsonRegistry<DummyRegistry> =
        EmbeddedJsonRegistry::new(RAW, Some("core.dummy_registry.v1"), "dummy");

    fn validate_v1(r: DummyRegistry) -> Result<DummyRegistry> {
        if r.version == 1 {
            Ok(r)
        } else {
            Err(Error::Validation(format!(
                "unsupported version {}",
                r.version
            )))
        }
    }

    #[test]
    fn embedded_loads_through_validate() {
        let reg = REG.load(validate_v1).expect("should load");
        assert_eq!(reg.version, 1);
    }

    #[test]
    fn config_extension_takes_precedence() {
        let mut config = FinstackConfig::default();
        let value = serde_json::json!({"version": 1});
        config
            .extensions
            .insert("core.dummy_registry.v1", value)
            .expect("static extension key");
        let reg = REG
            .load_from_config(&config, validate_v1)
            .expect("config-loaded registry");
        assert_eq!(reg.version, 1);
    }

    #[test]
    fn validation_failure_is_propagated() {
        let mut config = FinstackConfig::default();
        let value = serde_json::json!({"version": 99});
        config
            .extensions
            .insert("core.dummy_registry.v1", value)
            .expect("static extension key");
        let err = REG
            .load_from_config(&config, validate_v1)
            .expect_err("invalid version must fail validation");
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn document_type_can_differ_from_registry_type() {
        #[derive(Deserialize)]
        struct File {
            version: u32,
        }
        static SPLIT: EmbeddedJsonRegistry<DummyRegistry, File> =
            EmbeddedJsonRegistry::new(RAW, None, "split");
        let reg = SPLIT
            .load(|file| {
                Ok(DummyRegistry {
                    version: file.version,
                })
            })
            .expect("should load");
        assert_eq!(reg.version, 1);

        let mut config = FinstackConfig::default();
        config
            .extensions
            .insert("core.dummy_registry.v1", serde_json::json!({"version": 7}))
            .expect("static extension key");
        let without_key = SPLIT
            .load_from_config(&config, |file| {
                Ok(DummyRegistry {
                    version: file.version,
                })
            })
            .expect("no extension key means embedded fallback");
        assert_eq!(without_key.version, 1);
    }
}
