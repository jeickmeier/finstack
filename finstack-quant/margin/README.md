# finstack-quant-margin

Margin, collateral, and XVA types and calculators for Finstack. The crate is
standalone from `finstack-quant-valuations` so consumers can share agreement terms,
IM/VM engines, registry-backed defaults, and regulatory capital helpers without
pulling the full instrument stack.

## Workflows

- **OTC and repo margin** — CSA terms, VM parameters, IM methodology, eligible
  collateral schedules, repo maintenance rules (`OtcMarginSpec`, `RepoMarginSpec`).
- **Calculators** — variation margin, SIMM, BCBS-IOSCO schedule IM, repo haircut
  IM, and CCP proxy IM.
- **Regulatory capital** — FRTB sensitivity-based approach and SA-CCR EAD
  (`regulatory::frtb`, `regulatory::sa_ccr`).
- **XVA** — ISDA-style netting/collateral helpers and CVA/DVA/FVA/MVA
  integration over caller-supplied exposure profiles (`xva::{cva,mva,netting,types}`).

## Public modules

| Module | Role |
|--------|------|
| `types` | CSA, collateral, repo, SIMM, netting identifiers |
| `calculators` | VM and IM engines |
| `traits` | `Marginable` for consumer-crate integration |
| `metrics` | Utilization, excess collateral, funding cost, Haircut01 |
| `regulatory` | FRTB SBA and SA-CCR engines |
| `constants` | Shared heuristics |
| `xva` | Netting, CVA/DVA/FVA/MVA, and shared XVA types |

Registry JSON is embedded at build time. Overlays use the Finstack config
extension key `margin.registry.v1`;
but the registry and embedded data are owned by `finstack-quant-margin`. Factory
methods such as `CsaSpec::usd_regulatory()` and `OtcMarginSpec::usd_bilateral()`
resolve defaults from that registry.

## Quick examples

### Bilateral OTC spec

```rust,no_run
use finstack_quant_margin::{CsaSpec, OtcMarginSpec};

# fn main() -> finstack_quant_core::Result<()> {
let csa = CsaSpec::usd_regulatory()?;
let spec = OtcMarginSpec::bilateral_simm(csa);

assert!(spec.csa.requires_im());
assert_eq!(spec.vm_frequency.to_string(), "daily");
# Ok(())
# }
```

### SIMM from sensitivities

```rust,no_run
use finstack_quant_core::currency::Currency;
use finstack_quant_margin::{SimmCalculator, SimmSensitivities, SimmVersion};

# fn main() -> finstack_quant_core::Result<()> {
let calc = SimmCalculator::new(SimmVersion::V2_6)?;

let mut sensitivities = SimmSensitivities::new(Currency::USD);
sensitivities.add_ir_delta(Currency::USD, "5y", 50_000.0);
sensitivities.add_equity_delta("AAPL", 100_000.0);

let (total_im, breakdown) = calc.calculate_from_sensitivities(&sensitivities, Currency::USD);
assert!(total_im.amount() >= 0.0);
assert!(!breakdown.is_empty());
# Ok(())
# }
```

### Variation margin

```rust,no_run
use finstack_quant_core::currency::Currency;
use finstack_quant_core::money::Money;
use finstack_quant_margin::{CsaSpec, VmCalculator};
use time::Date;
use time::Month;

# fn main() -> finstack_quant_core::Result<()> {
let calc = VmCalculator::new(CsaSpec::usd_regulatory()?);

let exposure = Money::new(5_000_000.0, Currency::USD);
let posted = Money::new(3_000_000.0, Currency::USD);
let as_of = Date::from_calendar_date(2025, Month::January, 15)?;

let result = calc.calculate(exposure, posted, as_of)?;
assert!(result.delivery_amount.amount() >= 0.0 || result.return_amount.amount() >= 0.0);
# Ok(())
# }
```

### `Marginable` integration

```rust,no_run
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::Money;
use finstack_quant_margin::{Marginable, OtcMarginSpec, SimmSensitivities, SimmCalculator, SimmVersion};
use time::Month;

struct ExampleTrade {
    id: String,
    spec: OtcMarginSpec,
    mtm: Money,
    sensitivities: SimmSensitivities,
}

impl Marginable for ExampleTrade {
    fn id(&self) -> &str { &self.id }
    fn margin_spec(&self) -> Option<&OtcMarginSpec> { Some(&self.spec) }
    fn netting_set_id(&self) -> Option<finstack_quant_margin::NettingSetId> { None }
    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        _as_of: Date,
    ) -> finstack_quant_core::Result<SimmSensitivities> {
        Ok(self.sensitivities.clone())
    }
    fn mtm_for_vm(&self, _market: &MarketContext, _as_of: Date) -> finstack_quant_core::Result<Money> {
        Ok(self.mtm)
    }
}

# fn main() -> finstack_quant_core::Result<()> {
let spec = OtcMarginSpec::usd_bilateral()?;
let mut sensitivities = SimmSensitivities::new(Currency::USD);
sensitivities.add_ir_delta(Currency::USD, "5y", 25_000.0);

let trade = ExampleTrade {
    id: "SWAP-001".to_string(),
    spec,
    mtm: Money::new(1_000_000.0, Currency::USD),
    sensitivities,
};

let market = MarketContext::new();
let as_of = Date::from_calendar_date(2025, Month::January, 15)?;

let sens = trade.simm_sensitivities(&market, as_of)?;
let (im, _breakdown) = SimmCalculator::new(SimmVersion::V2_6)?
    .calculate_from_sensitivities(&sens, Currency::USD);
let vm = trade.mtm_for_vm(&market, as_of)?;
assert!(im.amount() >= 0.0);
assert!(vm.amount() >= 0.0);
# Ok(())
# }
```

## Conventions

- Rates, spreads, and haircuts are decimal fractions unless a field name says
  otherwise (for example `funding_spread_bp` in XVA config).
- VM/IM thresholds, MTAs, and independent amounts are `Money`.
- `Marginable::simm_sensitivities` expects currency-denominated risk measures
  (DV01/CS01-style), not raw quote moves.
- Schedule IM, cleared-IM proxy, and internal-model paths invoked through
  `ImCalculator` need `Marginable::im_exposure_base` (or an external CCP source);
  they do not fall back to MTM as notional.
- Persisted SIMM and FRTB tuple-keyed sensitivities use deterministic sorted
  entry arrays. Readers continue to accept the historical empty-object form,
  but non-empty tuple-keyed maps were never valid JSON objects.

## Embedded data

| File | Purpose |
|------|---------|
| `data/margin/defaults.v1.json` | Default VM, IM, timing, settlement |
| `data/margin/schedule_im.v1.json` | Schedule IM grids (e.g. `bcbs_iosco`) |
| `data/margin/collateral_schedules.v1.json` | Eligible collateral and haircuts |
| `data/margin/ccp_methodologies.v1.json` | CCP proxy rates and MPOR |
| `data/margin/simm.v1.json` | SIMM weights, correlations, concentration |
| `data/margin/xva_defaults.v1.json` | XVA exposure-grid and recovery defaults |
| `schemas/margin/1/margin.schema.json` | External margin-spec JSON schema |

Config overlay shape (extension key `margin.registry.v1`):

```json
{
  "extensions": {
    "margin.registry.v1": {
      "defaults": {
        "vm": {
          "threshold": 0.0,
          "mta": 250000.0
        }
      }
    }
  }
}
```

## Calculator notes

- **VM** — `VmCalculator` applies CSA threshold, MTA, rounding, and settlement
  dates from agreement terms.
- **SIMM** — `SimmCalculator` loads versioned parameters from the registry;
  `calculate_from_sensitivities` is the direct sensitivity path.
- **Schedule IM** — `ScheduleImCalculator::calculate_for_notional` applies the
  regulatory grid; the `ImCalculator` trait path uses `im_exposure_base`.
- **Cleared IM** — `ClearingHouseImCalculator` accepts external CCP values or
  scales `im_exposure_base` by registry-backed proxy rates.
- **Repo IM** — `HaircutImCalculator` with `RepoMarginSpec` (no-margin, MTM,
  net-exposure, triparty-style rules).

## XVA scope

`ExposureProfile` is a container: callers supply the EPE / ENE grid from their
own simulation or valuation stack, and this crate turns it into CVA, DVA, FVA,
and MVA. `apply_collateral_mpor` / `apply_variation_margin_mpor` model CSA
collateral with MPOR lag against the portfolio value at `t - mpor_days / 365`
when building a profile by hand. Exposure *generation* (rolling curves forward
and repricing instruments) is out of scope: it needs the pricing stack, and
`finstack-quant-margin` sits below `finstack-quant-valuations` in the
dependency order. Wrong-way risk and scenario carry remain out of scope.

## Verification

```bash
cargo test -p finstack-quant-margin
```

## References

- [ISDA SIMM](../../docs/REFERENCES.md#isda-simm)
- [ISDA 2016 VM CSA](../../docs/REFERENCES.md#isda-vm-csa-2016)
- [ISDA 2018 IM CSA](../../docs/REFERENCES.md#isda-im-csa-2018)
- [BCBS-IOSCO uncleared margin](../../docs/REFERENCES.md#bcbs-iosco-uncleared-margin)
- [Gregory XVA Challenge](../../docs/REFERENCES.md#gregory-xva-challenge)
- [Green XVA](../../docs/REFERENCES.md#green-xva)
- [BCBS 279 SA-CCR](../../docs/REFERENCES.md#bcbs-279-saccr)
