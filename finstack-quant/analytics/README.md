# finstack-quant-analytics

Portfolio performance and risk analytics on numeric return series and
`finstack_quant_core::dates::Date`, with no DataFrame or Polars dependency.

[`Performance`](src/performance/mod.rs) is the entry point. Construct it from
a price or return panel; scalars, drawdown statistics, rolling windows,
periodic returns, and benchmark-relative metrics are methods on that instance.

Per-domain modules (`returns`, `risk_metrics`, `drawdown`, `benchmark`,
`aggregation`, `lookback`) hold crate-internal building blocks that
`Performance` composes. Result and config types those modules define are
re-exported at the crate root because `Performance` returns them.

## Coverage

- **Returns**: simple returns, excess returns, compounded accumulation, geometric mean
- **Risk metrics**: CAGR, mean return, volatility, Sharpe, Sortino, downside deviation, Omega, gain-to-pain, modified Sharpe
- **Tail risk**: historical VaR, Expected Shortfall, parametric VaR, Cornish-Fisher VaR, skewness, kurtosis, tail ratios
- **Drawdown**: drawdown paths, episodes, max/mean drawdown, Ulcer Index, CDaR, Calmar, Martin, Sterling, Burke, Pain, recovery factor
- **Benchmark-relative**: tracking error, information ratio, beta (with SE and CI), alpha/beta/R² greeks, rolling greeks, up/down capture, batting average, Treynor, M-squared, multi-factor regression
- **Rolling series**: rolling Sharpe, Sortino, volatility, alpha/beta
- **Aggregation and lookbacks**: period compounding, win/loss streaks, Kelly criterion, MTD/QTD/YTD/FYTD range selection

## Dependencies

```toml
[dependencies]
finstack-quant-analytics = { path = "../finstack-quant/analytics" }
finstack-quant-core = { path = "../finstack-quant/core" }
```

Import path uses underscores even though the package name uses hyphens:

```rust
use finstack_quant_analytics::Performance;
use finstack_quant_core::dates::{Date, Month, PeriodKind};
```

A runnable construction example, plus return / drawdown / annualization
conventions, lives in the crate rustdoc (`cargo doc -p finstack-quant-analytics --open`).

## Public API

| Item | Module | Notes |
|------|--------|-------|
| `Performance`, `LookbackReturns` | `performance` | Entry point |
| `PeriodStats` | `aggregation` | Returned by `Performance::period_stats` |
| `DrawdownEpisode` | `drawdown` | Returned by `Performance::drawdown_details` |
| `BetaResult`, `GreeksResult`, `RollingGreeks`, `MultiFactorResult` | `benchmark` | Returned by benchmark methods on `Performance` |
| `DatedSeries` | `risk_metrics` | Returned by `Performance::rolling_*` |
| `beta` | `benchmark` | Freestanding OLS beta; also used by `finstack-quant-valuations` |
| `correlation` | `correlation` | Shared row-major correlation validation / repair infrastructure used by valuations and factor-model crates |

All other analytics building-block functions are crate-internal (`pub(crate)`).

## Numerical behavior

- Compounding uses compensated summation in log space for long-series stability.
- `Performance::new` and `Performance::from_returns` reject empty inputs, ragged matrices, unknown benchmark names, duplicate or non-monotonic dates, non-finite values, and interior invalid returns.
- Multi-factor regression rejects mismatched factor lengths, non-finite factors, non-positive annualization factors, and singular or near-singular factor matrices.
- Volatility, covariance, skewness, and kurtosis use sample statistics (`n - 1` denominator).
- Degenerate cases return `0.0`, `NaN`, or `±∞` rather than panicking.

## Serialization

`Performance`, `LookbackReturns`, `PeriodStats`, `DrawdownEpisode`, `BetaResult`, `GreeksResult`, `MultiFactorResult`, `RollingGreeks`, and `DatedSeries` derive `Serialize`/`Deserialize`.

## Bindings

- Python: flat performance surface under `finstack_quant.analytics`; shared correlation utilities are bound under `finstack_quant.valuations.correlation` for historical namespace compatibility. See `finstack-quant-py/parity_contract.toml`.
- WASM: mirrors `Performance`; result types serialize to JS objects via `serde-wasm-bindgen`.

## References

Quantitative references: [`docs/REFERENCES.md`](../../docs/REFERENCES.md).

## Verification

```bash
cargo fmt -p finstack-quant-analytics
cargo clippy -p finstack-quant-analytics --all-features -- -D warnings
cargo test -p finstack-quant-analytics
cargo test -p finstack-quant-analytics --doc
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-quant-analytics --no-deps --all-features
```
