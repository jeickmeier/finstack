//! Shared serial/parallel mapping so rayon stays off the wasm32 graph.

use crate::types::result::ExecutionPolicy;

/// Map `items` with `f`, using Rayon only on native `ExecutionPolicy::Parallel`.
pub(crate) fn map_policy<T, R, F>(policy: ExecutionPolicy, items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        match policy {
            ExecutionPolicy::Parallel => items.par_iter().map(f).collect(),
            ExecutionPolicy::Serial => items.iter().map(f).collect(),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = policy;
        items.iter().map(f).collect()
    }
}

/// Fallible map of `items` with `f`, using Rayon only on native `Parallel`.
pub(crate) fn try_map_policy<T, R, E, F>(
    policy: ExecutionPolicy,
    items: &[T],
    f: F,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
    F: Fn(&T) -> Result<R, E> + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        match policy {
            ExecutionPolicy::Parallel => items.par_iter().map(f).collect(),
            ExecutionPolicy::Serial => items.iter().map(f).collect(),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = policy;
        items.iter().map(f).collect()
    }
}

/// Fallible zip-map of two slices, using Rayon only on native `Parallel`.
pub(crate) fn try_map_policy_zip<A, B, R, E, F>(
    policy: ExecutionPolicy,
    left: &[A],
    right: &[B],
    f: F,
) -> Result<Vec<R>, E>
where
    A: Sync,
    B: Sync,
    R: Send,
    E: Send,
    F: Fn((&A, &B)) -> Result<R, E> + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        match policy {
            ExecutionPolicy::Parallel => left.par_iter().zip(right.par_iter()).map(f).collect(),
            ExecutionPolicy::Serial => left.iter().zip(right.iter()).map(f).collect(),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = policy;
        left.iter().zip(right.iter()).map(f).collect()
    }
}
