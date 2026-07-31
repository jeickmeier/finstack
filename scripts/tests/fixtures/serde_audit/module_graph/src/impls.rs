use crate::public_api::CrossFileResult;

impl serde::Serialize for CrossFileResult {}
impl<'de> serde::Deserialize<'de> for CrossFileResult {}
impl schemars::JsonSchema for CrossFileResult {}

impl serde::Serialize for crate::public_api::duplicate_a::DuplicateResult {}
impl<'de> serde::Deserialize<'de> for crate::public_api::duplicate_a::DuplicateResult {}
impl schemars::JsonSchema for crate::public_api::duplicate_a::DuplicateResult {}

impl serde::Serialize for crate::public_api::ManualMarker {}
impl<'de> serde::Deserialize<'de> for crate::public_api::ManualMarker {}

#[cfg(test)]
impl serde::Serialize for crate::public_api::CfgManualResult {}

#[cfg(feature = "optional")]
impl<'de> serde::Deserialize<'de> for crate::public_api::CfgManualResult {}
