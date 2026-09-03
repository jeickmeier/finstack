use super::MetricId;
use finstack_quant_core::HashMap;
use std::fmt::Write as _;

impl MetricId {
    /// Build a flattened composite metric key from a base ID and components.
    ///
    /// The wire format is `base::component[::component...]` and is also the
    /// human-facing form: ordinary identifiers such as `USD-OIS`, `usd_ois`
    /// or `10y` are written literally, so a key reads exactly as documented
    /// (`bucketed_dv01::USD-OIS::10y`, `pv01::USD-OIS`). Only what would
    /// break the framing is escaped as `_xHH`: the `:` delimiter byte, an
    /// underscore immediately followed by `x` (so a literal `_x` can never be
    /// mistaken for an escape marker), and the reserved `_empty` spelling. The
    /// empty component is encoded as `_empty`. [`Self::decode_components`] is
    /// the exact inverse and additionally accepts the legacy fully-escaped
    /// form (`USD_x2dOIS`) that earlier releases persisted.
    ///
    /// # Arguments
    ///
    /// * `base` - Base metric the components qualify (for example
    ///   `MetricId::BucketedDv01` or `MetricId::Pv01`).
    /// * `components` - Ordered coordinate labels (curve ids, bucket labels,
    ///   row/column labels) appended after the base, in order.
    pub fn composite(base: &MetricId, components: &[&str]) -> Self {
        let mut key = String::with_capacity(base.as_str().len() + components.len() * 8);
        key.push_str(base.as_str());

        for component in components {
            key.push_str("::");
            if component.is_empty() {
                key.push_str("_empty");
                continue;
            }
            if *component == "_empty" {
                key.push_str("_x5fempty");
                continue;
            }
            for (index, ch) in component.char_indices() {
                let escape =
                    ch == ':' || (ch == '_' && component[index + ch.len_utf8()..].starts_with('x'));
                if escape {
                    // Both `:` and `_` are single-byte ASCII.
                    let _ = write!(&mut key, "_x{:02x}", ch as u32);
                } else {
                    key.push(ch);
                }
            }
        }
        Self::custom(key)
    }

    /// Decode this metric's components when it is a composite of `base`.
    ///
    /// Returns `None` for the scalar base metric or a different base.
    /// Canonical `_xHH` and `_empty` encodings are decoded. A component with a
    /// malformed escape marker or an escape sequence that does not produce
    /// valid UTF-8 is returned literally so legacy persisted keys are never
    /// omitted.
    ///
    /// A valid-looking `_xHH` substring is fundamentally ambiguous because the
    /// historical wire format has no explicit whole-component marker. Use
    /// [`Self::decode_series_components`] when decoding multiple keys; it
    /// resolves decoded-coordinate collisions without dropping entries.
    ///
    /// # Arguments
    ///
    /// * `base` - Base metric whose composite keys are being decoded (for
    ///   example `MetricId::BucketedDv01`); the key must start with
    ///   `"{base}::"` or `None` is returned.
    pub fn decode_components(&self, base: &MetricId) -> Option<Vec<String>> {
        let suffix = self
            .as_str()
            .strip_prefix(base.as_str())?
            .strip_prefix("::")?;

        Some(suffix.split("::").map(decode_component).collect())
    }

    /// Decode a sequence of composite metric keys without losing collisions.
    ///
    /// The returned vector is aligned one-for-one with `metrics`; scalar and
    /// non-matching keys produce `None`. Unambiguous keys use
    /// [`Self::decode_components`]. If two or more distinct wire keys resolve
    /// to the same component vector, every entry in that collision group uses
    /// its literal wire components instead. Collision detection repeats to a
    /// fixed point because a literal fallback can collide with another key's
    /// decoded coordinates. This preserves all values and their insertion
    /// order without rewriting persisted keys or inventing a new wire marker.
    ///
    /// # Arguments
    ///
    /// * `base` - Base date or base currency anchoring relative quotes and curves
    /// * `metrics` - Requested metric identifiers evaluated and returned in order.
    pub fn decode_series_components<'a>(
        base: &MetricId,
        metrics: impl IntoIterator<Item = &'a MetricId>,
    ) -> Vec<Option<Vec<String>>> {
        let parsed: Vec<Option<(Vec<String>, Vec<String>)>> = metrics
            .into_iter()
            .map(|metric| {
                let suffix = metric
                    .as_str()
                    .strip_prefix(base.as_str())?
                    .strip_prefix("::")?;
                let literal = suffix.split("::").map(str::to_string).collect();
                let decoded = metric.decode_components(base)?;
                Some((literal, decoded))
            })
            .collect();

        let mut resolved: Vec<Option<Vec<String>>> = parsed
            .iter()
            .map(|entry| entry.as_ref().map(|(_, decoded)| decoded.clone()))
            .collect();

        loop {
            let mut counts: HashMap<Vec<String>, usize> = HashMap::default();
            for components in resolved.iter().flatten() {
                *counts.entry(components.clone()).or_default() += 1;
            }

            let mut changed = false;
            for (entry, components) in parsed.iter().zip(&mut resolved) {
                let (Some((literal, _)), Some(current)) = (entry, components) else {
                    continue;
                };
                if counts.get(current).copied().unwrap_or_default() > 1 && current != literal {
                    current.clone_from(literal);
                    changed = true;
                }
            }

            if !changed {
                return resolved;
            }
        }
    }
}

fn decode_component(component: &str) -> String {
    if component == "_empty" {
        return String::new();
    }

    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if index + 3 < bytes.len() && bytes[index] == b'_' && bytes[index + 1] == b'x' {
            let (Some(high), Some(low)) =
                (decode_hex(bytes[index + 2]), decode_hex(bytes[index + 3]))
            else {
                return component.to_string();
            };
            decoded.push((high << 4) | low);
            index += 4;
        } else if bytes[index] == b'_' && index + 1 < bytes.len() && bytes[index + 1] == b'x' {
            return component.to_string();
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| component.to_string())
}

const fn decode_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
