//! Shared panel grouping: entity sort and time-key partitions.

use finstack_quant_core::Result;
use std::collections::BTreeMap;

/// Sort row indices by `entity`, then lexicographic `order`, then input index.
pub(crate) fn sorted_indices(entity: &[String], order: &[String]) -> Vec<usize> {
    let mut indices = (0..entity.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        entity[*left]
            .cmp(&entity[*right])
            .then(order[*left].cmp(&order[*right]))
            .then(left.cmp(right))
    });
    indices
}

/// Visit each contiguous entity run in an already-sorted index.
pub(crate) fn try_for_each_entity(
    entity: &[String],
    indices: &[usize],
    mut visit: impl FnMut(&[usize]) -> Result<()>,
) -> Result<()> {
    let mut start = 0;
    while start < indices.len() {
        let mut end = start + 1;
        while end < indices.len() && entity[indices[end]] == entity[indices[start]] {
            end += 1;
        }
        visit(&indices[start..end])?;
        start = end;
    }
    Ok(())
}

/// Partition row indices by a single opaque key.
pub(crate) fn partition_by_key(keys: &[String]) -> BTreeMap<&str, Vec<usize>> {
    let mut partitions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (idx, key) in keys.iter().enumerate() {
        partitions.entry(key.as_str()).or_default().push(idx);
    }
    partitions
}

/// Partition row indices by a `(first, second)` key pair without concatenation.
pub(crate) fn partition_by_pair<'a>(
    first: &'a [String],
    second: &'a [String],
) -> BTreeMap<(&'a str, &'a str), Vec<usize>> {
    let mut partitions: BTreeMap<(&'a str, &'a str), Vec<usize>> = BTreeMap::new();
    for (idx, (a, b)) in first.iter().zip(second.iter()).enumerate() {
        partitions
            .entry((a.as_str(), b.as_str()))
            .or_default()
            .push(idx);
    }
    partitions
}
