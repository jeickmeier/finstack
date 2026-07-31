pub mod public_api;
mod hidden;
mod impls;
mod glob_bridge;
#[cfg(feature = "enabled")]
mod default_private;
#[cfg(feature = "optional")]
mod optional_private;
#[cfg(feature = "optional")]
mod optional_impls;
#[cfg(feature = "enabled")]
pub mod enabled_public;
#[cfg(not(feature = "enabled"))]
pub mod disabled_public;

pub use hidden::ReexportedResult;
pub use hidden::ReexportedResult as RenamedResult;
pub use hidden::CashflowResult;
pub use hidden::AliasOnlyResult as PublicAliasResult;
pub use glob_bridge::*;
#[cfg(feature = "enabled")]
pub use default_private::*;
#[cfg(feature = "optional")]
pub use optional_private::*;
#[cfg(feature = "optional")]
pub use hidden::AliasOnlyResult as OptionalAliasResult;
#[cfg(not(feature = "enabled"))]
pub use hidden::AliasOnlyResult as DisabledAliasResult;
pub type RootAliasSpec = public_api::AliasTarget;

pub mod inline_a {
    #[derive(serde::Serialize)]
    pub struct InlineDuplicateResult {
        pub value: String,
    }
}

pub mod inline_b {
    #[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct InlineDuplicateResult {
        pub value: String,
    }
}

#[cfg(feature = "enabled")]
pub mod enabled_inline {
    #[derive(serde::Serialize)]
    pub struct EnabledInlineResult {
        pub value: String,
    }
}

#[cfg(feature = "optional")]
pub mod optional_inline {
    #[derive(serde::Serialize)]
    pub struct OptionalInlineResult {
        pub value: String,
    }
}

const RAW_SOURCE: &str = r###"
pub struct PhantomSpec {
    pub schema_version: u32,
}
"###;
