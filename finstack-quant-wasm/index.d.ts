// Type declarations for the finstack-quant-wasm namespaced facade.
// Shapes follow `wasm-bindgen` JS names in `src/api/**` (see Rust `js_name`).
// The raw `pkg/finstack_quant_wasm.d.ts` emitted by wasm-bindgen is intentionally
// not the package root contract: it exposes a flat module, while `index.js`
// publishes a namespaced facade. Keep this file as the facade declaration and
// use generated `types/generated/*` files only for JSON envelope shapes.
//
// Building a MarketContext from quotes (canonical path):
//
//   import { valuations } from 'finstack-quant-wasm/exports/valuations.js';
//   import type { CalibrationEnvelope } from 'finstack-quant-wasm';
//   const envelope: CalibrationEnvelope = {
//     schema: 'finstack_quant.calibration/1',
//     plan: { id: 'usd_curves', quote_sets: {...}, steps: [...], settings: {} },
//     market_data: [],   // flat id-addressable quotes/snapshots
//     prior_market: [],  // optional pre-built curves/surfaces
//   };
//   const result = valuations.calibrate(envelope);  // CalibrationResultEnvelope
//   const marketJson = JSON.stringify(result.result.final_market);
//
// `result.result.final_market` is the materialized MarketContextState ready
// for any downstream pricing / scenario / attribution call that takes a
// market_json argument. Always check the per-step report
// (`result.result.step_reports`) and the plan summary
// (`result.result.report`) to confirm the curves actually fit before using
// the market downstream.
//
// `validateCalibrationJson` is a fast pre-flight check that canonicalizes
// the envelope without solving — use it to surface schema errors early.
//
// Phase 4 diagnostics: errors thrown by `calibrate`,
// `validateCalibrationJson`, `dryRun`, and `dependencyGraphJson` have:
//   - name: 'CalibrationEnvelopeError'
//   - cause: structured EnvelopeError payload (object with `kind` etc.)
// Standard try/catch exposes both via `e.name` and `e.cause`.

// WASM ownership: every wasm-bindgen class exposed below owns a wasm heap
// allocation. Call `free()` when a handle is no longer needed. On runtimes
// that define `Symbol.dispose`, wasm-bindgen also installs
// `instance[Symbol.dispose] === instance.free`. Plain JSON results, arrays,
// and namespace functions do not need manual disposal.

/**
 * Inputs accepted by the wasm-bindgen web initializer.
 */
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

/**
 * Initialized WebAssembly exports.
 */
export type InitOutput = WebAssembly.Exports;

/**
 * Initialize the package's WebAssembly module.
 * @example
 * ```typescript
 * import init from "finstack-quant-wasm";
 * const wasm = await init();
 * void wasm;
 * ```
 * @param moduleOrPath - Optional module source: a URL, Response, WebAssembly.Module, or Promise accepted by wasm-bindgen initialization.
 * @returns Returns a Promise that resolves to `InitOutput`.
 */
export default function init(
  moduleOrPath?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>
): Promise<InitOutput>;

// --- Calibration envelope types (generated from Rust via ts-rs) ---
import type { CalibrationEnvelope } from './types/generated/CalibrationEnvelope';
import type { CalibrationResultEnvelope } from './types/generated/CalibrationResultEnvelope';
import type { MaterializationReport } from './types/generated/MaterializationReport';
import type { ValidationReport } from './types/generated/ValidationReport';

export type { CalibrationEnvelope, CalibrationResultEnvelope };
export type { Diagnostic } from './types/generated/Diagnostic';
export type { MaterializationPhases } from './types/generated/MaterializationPhases';
export type { MaterializationReport } from './types/generated/MaterializationReport';
export type { ValidationReport } from './types/generated/ValidationReport';
export type { CalibrationPlan } from './types/generated/CalibrationPlan';
export type { CalibrationStep } from './types/generated/CalibrationStep';
export type { StepParams } from './types/generated/StepParams';
export type { MarketDatum } from './types/generated/MarketDatum';
export type { PriorMarketObject } from './types/generated/PriorMarketObject';
export type { CalibrationResult } from './types/generated/CalibrationResult';
export type { CalibrationReport } from './types/generated/CalibrationReport';

// --- core -----------------------------------------------------------------

/**
 * Lifecycle contract for a WebAssembly-backed value that owns a wasm heap allocation.
 */
export interface WasmOwned {
  /**
   * Release the underlying wasm heap allocation. Do not use this handle afterward.
   */
  free(): void;
}

// wasm-bindgen emits these as classes. Interface merging adds their generated
// `free()` contract without duplicating methods. At runtime, wasm-bindgen also
// installs `[Symbol.dispose]` as an alias of `free` when the host defines that
// symbol; it is intentionally omitted here so ES2020 consumers do not require
// the `esnext.disposable` TypeScript library.
/**
 * Stateful performance analytics engine over a panel of ticker price (or return) series.
 *
 * Dates are ISO-8601 values in ascending order. Numeric inputs are row-major
 * with one row per date and one column per ticker. Scalar rates and returns
 * use decimal fractions; numeric outputs are Float64Array values in ticker
 * order unless the method documents an object or matrix shape.
 *
 * Invalid dates, shapes, frequencies, tickers, and confidence levels are
 * returned as rejected JsValue errors.
 */
export interface Performance extends WasmOwned {}
/**
 * Calibrated credit factor hierarchy artifact.
 *
 * Produced by [`JsCreditCalibrator`] or loaded from JSON via
 * [`JsCreditFactorModel::from_json`]. Immutable once constructed.
 */
export interface CreditFactorModel extends WasmOwned {}
/**
 * Deterministic calibrator that produces a [`JsCreditFactorModel`].
 *
 * Configuration and inputs are passed as JSON strings.
 */
export interface CreditCalibrator extends WasmOwned {}
/**
 * Snapshot of all hierarchy-level factor values at a single date.
 *
 * Produced by [`decompose_levels`]. Pass to [`decompose_period`] to compute
 * period-over-period changes.  The full data is available via `toJson`.
 */
export interface LevelsAtDate extends WasmOwned {}
/**
 * Component-wise difference between two [`JsLevelsAtDate`] snapshots.
 *
 * Produced by [`decompose_period`].
 */
export interface PeriodDecomposition extends WasmOwned {}
/**
 * Vol-forecast view over a calibrated `CreditFactorModel`.
 *
 * `VolHorizon::Custom` is intentionally **not** exposed.
 */
export interface FactorCovarianceForecast extends WasmOwned {}
/**
 * Opaque handle wrapping a parsed [`MarketContext`].
 *
 * Construct once from JSON, then pass to `priceInstrumentWithMarket`,
 * `priceInstrumentWithMetricsAndMarket`, etc.  Eliminates the per-call
 * market-parse overhead in bulk-pricing and Greeks-sweep loops.
 *
 * @example
 * ```javascript
 * const market = new valuations.Market(marketJson);
 * for (const instr of instruments) {
 *   const result = valuations.instruments.priceInstrumentWithMarket(instr, market, "2025-06-15", "default");
 * }
 * ```
 */
export interface Market extends WasmOwned {}
/**
 * Handle to a built [`finstack_quant_portfolio::Portfolio`] that can be reused
 * across WASM calls without re-parsing and rebuilding from the spec.
 *
 * `Portfolio::from_spec` parses positions, builds indices, and validates
 * invariants; for pipelines that call both `valuePortfolio` and
 * `aggregateFullCashflows` on the same portfolio, holding this handle
 * avoids paying that cost twice.
 */
export interface Portfolio extends WasmOwned {}

/**
 * ISO-4217 currency code wrapper for JavaScript.
 *
 * Currencies parse from three-letter alphabetic codes (case-insensitive).
 * They expose the alphabetic code, the ISO numeric code, and the number of
 * decimal places (minor units) for the currency.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const usd = new core.Currency("USD");
 * usd.code;     // "USD"
 * usd.numeric;  // 840
 * usd.decimals; // 2
 * ```
 */
export interface Currency extends WasmOwned {
  /**
   * Three-letter ISO-4217 alphabetic code.
   *
   * @returns The uppercase alphabetic code (e.g. `"USD"`).
   */
  readonly code: string;
  /**
   * ISO-4217 numeric code.
   *
   * @returns Numeric code (e.g. `840` for USD, `978` for EUR).
   */
  readonly numeric: number;
  /**
   * Number of decimal places (minor units) for this currency.
   *
   * @returns Decimal-place count (e.g. `2` for USD, `0` for JPY).
   */
  readonly decimals: number;
  /**
   * Human-readable code (same as `code`).
   *
   * @returns The uppercase alphabetic ISO-4217 code.
   */
  toString(): string;
  /**
   * Serialize to a JSON string.
   *
   * @returns A JSON string (the ISO-4217 alphabetic code in quotes).
   * @throws If serialization fails (should not happen for valid `Currency`).
   */
  toJson(): string;
}

/**
 * ISO-4217 currency code wrapper for JavaScript.
 *
 * Currencies parse from three-letter alphabetic codes (case-insensitive).
 * They expose the alphabetic code, the ISO numeric code, and the number of
 * decimal places (minor units) for the currency.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const usd = new core.Currency("USD");
 * usd.code;     // "USD"
 * usd.numeric;  // 840
 * usd.decimals; // 2
 * ```
 */
export interface CurrencyConstructor {
  /**
   * Parse a case-insensitive ISO-4217 alphabetic currency code.
   *
   * @example
   * ```javascript
   * const eur = new core.Currency("eur"); // case-insensitive
   * eur.code; // "EUR"
   * ```
   * @param code - Three-letter ISO-4217 code (e.g. `"USD"`, `"eur"`,
   * @returns Constructed `Currency`.
   * @throws If `code` is not a recognized ISO-4217 alphabetic code.
   */
  new (code: string): Currency;
  /**
   * Deserialize from a JSON string produced by `Currency.toJson`.
   *
   * @param json - A JSON string containing a quoted ISO-4217 code.
   * @returns The parsed `Currency`.
   * @throws If `json` is malformed or contains an unknown code.
   */
  fromJson(json: string): Currency;
}

/**
 * Currency-tagged monetary amount.
 *
 * Money values pin a numeric amount to a [`JsCurrency`]. Arithmetic
 * (`add`, `sub`) refuses to mix currencies; scalar multiplication and
 * division preserve the currency.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const usd = new core.Currency("USD");
 * const total = new core.Money(1_000_000, usd);
 * const fee   = new core.Money(50, usd);
 * const net   = total.sub(fee);                 // Money { amount: 999950, currency: USD }
 * const tax   = net.mulScalar(0.07);            // 7% of net
 * console.log(net.toString(), tax.toString());  // "USD 999950.00", "USD 69996.50"
 * ```
 */
export interface Money extends WasmOwned {
  /**
   * Numeric amount in major units as `f64`.
   *
   * The Rust core stores money as `Decimal`; this getter exposes the finite
   * JavaScript number view for interop.
   *
   * @returns Amount in major units (e.g. dollars, not cents).
   */
  readonly amount: number;
  /**
   * Currency of this amount.
   *
   * @returns The [`JsCurrency`] this amount is tagged with.
   */
  readonly currency: Currency;
  /**
   * Lossless amount as a decimal string (e.g. `"1234.56"`).
   *
   * Renders the internal Rust `Decimal` directly, so no `f64` round-trip
   * occurs. Parse with a JavaScript decimal library for exact arithmetic.
   *
   * @returns The exact decimal amount as a string.
   */
  amountDecimal(): string;
  /**
   * Convert using an already-resolved positive FX rate.
   * @returns Returns the resulting `Money` value or WebAssembly handle.
   * @param target - Target Currency for the converted monetary amount.
   * @param rate - FX conversion rate expressed as target-currency units per source-currency unit.
   * @throws Error - For a different target currency, throws a JavaScript exception if `rate` is non-finite or not strictly positive, or if the converted amount cannot be represented as a decimal.
   */
  convertAtRate(target: Currency, rate: number): Money;
  /**
   * Add two amounts.
   *
   * @example
   * ```javascript
   * const usd = new core.Currency("USD");
   * const a = new core.Money(10, usd);
   * const b = new core.Money(5, usd);
   * a.add(b).amount;  // 15
   * ```
   * @param other - Another `Money` value.
   * @returns Sum, in the same currency.
   * @throws If `other.currency` differs from `this.currency`, or the
   */
  add(other: Money): Money;
  /**
   * Subtract two amounts.
   *
   * @param other - Another `Money` value.
   * @returns Difference, in the same currency.
   * @throws If `other.currency` differs from `this.currency`, or the
   */
  sub(other: Money): Money;
  /**
   * Multiply by a scalar.
   *
   * @param factor - Dimensionless multiplier (must be finite).
   * @returns Scaled amount, in the same currency.
   * @throws If `factor` is non-finite or the result is not representable.
   */
  mulScalar(factor: number): Money;
  /**
   * Divide by a scalar.
   *
   * @param divisor - Dimensionless divisor (must be finite and non-zero).
   * @returns Scaled amount, in the same currency.
   * @throws If `divisor` is zero, non-finite, or the result is not representable.
   */
  divScalar(divisor: number): Money;
  /**
   * Negate the monetary amount.
   *
   * @returns Negated amount in the same currency.
   * @throws If the negation is not representable as a `Decimal`.
   */
  negate(): Money;
  /**
   * Default string representation (e.g. `"USD 10.00"`).
   *
   * @returns Formatted amount with currency code.
   */
  toString(): string;
}

/**
 * Currency-tagged monetary amount.
 *
 * Money values pin a numeric amount to a [`JsCurrency`]. Arithmetic
 * (`add`, `sub`) refuses to mix currencies; scalar multiplication and
 * division preserve the currency.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const usd = new core.Currency("USD");
 * const total = new core.Money(1_000_000, usd);
 * const fee   = new core.Money(50, usd);
 * const net   = total.sub(fee);                 // Money { amount: 999950, currency: USD }
 * const tax   = net.mulScalar(0.07);            // 7% of net
 * console.log(net.toString(), tax.toString());  // "USD 999950.00", "USD 69996.50"
 * ```
 */
export interface MoneyConstructor {
  /**
   * Creates a new money value without implicit currency-minor-unit rounding.
   *
   * WASM accepts a JavaScript `number` only. Its finite numeric value is
   * converted to Rust `Decimal` and stored without currency-minor-unit
   * rounding; precision already absent from the input `number` cannot be
   * recovered. Formatting does not mutate the stored amount.
   *
   * @example
   * ```javascript
   * const usd = new core.Currency("USD");
   * const m = new core.Money(1234.56, usd);
   * m.amount;          // 1234.56
   * m.currency.code;   // "USD"
   * ```
   * @param amount - Numeric amount in major units (must be finite).
   * @param currency - ISO-4217 Currency object that tags the amount and controls arithmetic compatibility.
   * @returns The constructed `Money`.
   * @throws If `amount` is non-finite (NaN, ±∞) or cannot be represented as a `Decimal`.
   */
  new (amount: number, currency: Currency): Money;
}

/**
 * Interest or discount rate stored as a decimal (e.g. `0.05` is 5%).
 *
 * Conventions:
 * - **Decimal**: `0.05` represents 5%.
 * - **Percent**: `5.0` represents 5%.
 * - **Basis points**: `500` represents 5% (1 bp = 0.01%).
 *
 * Use the `fromPercent` or `fromBp` factories to avoid scaling errors
 * when working with quoted rates.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const r = core.Rate.fromBp(250);     // 2.5% as 250 bp
 * r.asDecimal;  // 0.025
 * r.asPercent;  // 2.5
 * r.asBp;      // 250
 * ```
 */
export interface Rate extends WasmOwned {
  /**
   * Rate as a decimal (e.g. `0.05` for 5%).
   *
   * @returns Decimal rate.
   */
  readonly asDecimal: number;
  /**
   * Rate as a percent (e.g. `5.0` for 5%).
   *
   * @returns Percent rate.
   */
  readonly asPercent: number;
  /**
   * Rate in basis points, rounded to the nearest integer (e.g. `500` for 5%).
   *
   * @returns Rate in bp.
   */
  readonly asBp: number;
}

/**
 * Interest or discount rate stored as a decimal (e.g. `0.05` is 5%).
 *
 * Conventions:
 * - **Decimal**: `0.05` represents 5%.
 * - **Percent**: `5.0` represents 5%.
 * - **Basis points**: `500` represents 5% (1 bp = 0.01%).
 *
 * Use the `fromPercent` or `fromBp` factories to avoid scaling errors
 * when working with quoted rates.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const r = core.Rate.fromBp(250);     // 2.5% as 250 bp
 * r.asDecimal;  // 0.025
 * r.asPercent;  // 2.5
 * r.asBp;      // 250
 * ```
 */
export interface RateConstructor {
  /**
   * Create a rate from a decimal value.
   *
   * @example
   * ```javascript
   * const r = new core.Rate(0.05);  // 5%
   * r.asPercent;  // 5
   * ```
   * @param decimal - Rate as a decimal (e.g. `0.05` for 5%).
   * @returns The constructed `Rate`.
   * @throws If `decimal` is non-finite (NaN, ±∞).
   */
  new (decimal: number): Rate;
  /**
   * Create a rate from a percent figure.
   *
   * @example
   * ```javascript
   * const r = core.Rate.fromPercent(5.0);
   * r.asDecimal;  // 0.05
   * ```
   * @param pct - Percent value (e.g. `5.0` for 5%).
   * @returns The constructed `Rate`.
   * @throws If `pct` is non-finite.
   */
  fromPercent(pct: number): Rate;
  /**
   * Create a rate from a whole number of basis points.
   *
   * The canonical Rust `Rate::from_bp` takes an integer (`i32`) number
   * of basis points. Because JavaScript numbers are `f64`, this binding
   * accepts a float but **rejects fractional input** rather than
   * silently rounding it: a sub-bp rate quietly rounded to whole bp is a
   * pricing bug, not a convenience. Use `new Rate(decimal)` or
   * `Rate.fromPercent` for sub-bp rates.
   *
   * @example
   * ```javascript
   * const r = core.Rate.fromBp(250);  // 2.5%
   * r.asDecimal;  // 0.025
   * ```
   * @param bp - Rate in whole basis points (e.g. `500` for 5%).
   * @returns The constructed `Rate`.
   * @throws If `bp` is non-finite or not a whole number of basis points.
   */
  fromBp(bp: number): Rate;
}

/**
 * Basis points (1 bp = 0.01%, 10_000 bp = 100%).
 *
 * Stored as integer bp internally; constructors reject fractional input.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const spread = new core.Bps(125);
 * spread.asDecimal();  // 0.0125
 * spread.asBp();      // 125
 * ```
 */
export interface Bps extends WasmOwned {
  /**
   * Value as a decimal (e.g. 25 bp → 0.0025).
   *
   * @returns Decimal equivalent.
   */
  asDecimal(): number;
  /**
   * Value in whole basis points.
   *
   * @returns Integer bp.
   */
  asBp(): number;
}

/**
 * Basis points (1 bp = 0.01%, 10_000 bp = 100%).
 *
 * Stored as integer bp internally; constructors reject fractional input.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const spread = new core.Bps(125);
 * spread.asDecimal();  // 0.0125
 * spread.asBp();      // 125
 * ```
 */
export interface BpsConstructor {
  /**
   * Create basis points from a whole-number value.
   *
   * @param value - Value in whole basis points (e.g. `25` for 25 bp).
   * @returns The constructed `Bps`.
   * @throws If `value` is non-finite or not a whole number of basis
   */
  new (value: number): Bps;
}

/**
 * Percentage stored in percent points (`5.0` means 5%).
 *
 * Use this when you want the API to be explicit that the value is in
 * percent (rather than decimal). Equivalent to `Rate` for arithmetic.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const p = new core.Percentage(5.0);
 * p.asDecimal();  // 0.05
 * p.asPercent();  // 5
 * ```
 */
export interface Percentage extends WasmOwned {
  /**
   * Value as a decimal (5% → 0.05).
   *
   * @returns Decimal equivalent.
   */
  asDecimal(): number;
  /**
   * Value in percent points.
   *
   * @returns Percent value.
   */
  asPercent(): number;
}

/**
 * Percentage stored in percent points (`5.0` means 5%).
 *
 * Use this when you want the API to be explicit that the value is in
 * percent (rather than decimal). Equivalent to `Rate` for arithmetic.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const p = new core.Percentage(5.0);
 * p.asDecimal();  // 0.05
 * p.asPercent();  // 5
 * ```
 */
export interface PercentageConstructor {
  /**
   * Create a percentage.
   *
   * @param value - Value in percent (e.g. `5.0` for 5%).
   * @returns The constructed `Percentage`.
   * @throws If `value` is non-finite.
   */
  new (value: number): Percentage;
}

/**
 * Day-count convention for computing year fractions and day counts.
 *
 * Dates are represented as **epoch days** (`i32`, days since 1970-01-01).
 * Use `createDate` to convert from a `(year, month, day)` triple.
 *
 * Available conventions and their factories:
 * - `act_360` → `DayCount.act360`
 * - `act_365f` → `DayCount.act365f`
 * - `30_360` → `DayCount.thirty360`
 * - `30e_360` → `DayCount.thirtyE360`
 * - `30e_360_isda` → `DayCount.thirtyE360Isda`
 * - `act_act` (ISDA) → `DayCount.actAct`
 * - `act_act_isma` (ICMA) → `DayCount.actActIsma`
 * - `bus_252` → `DayCount.bus252`
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const day_count = core.DayCount.act365f();
 * const start = core.createDate(2025, 1, 15);
 * const end   = core.createDate(2025, 7, 15);
 * const yf    = day_count.yearFraction(start, end);
 * // yf ≈ 0.4959 (181 / 365)
 * ```
 */
export interface DayCount extends WasmOwned {
  /**
   * Compute the year fraction between two dates given as epoch days.
   *
   * Act/Act ISMA and Bus/252 require explicit frequency/calendar context.
   * This method throws for those conventions; call
   * `DayCount.yearFractionWithContext` with a configured `DayCountContext`.
   *
   * @example
   * ```javascript
   * const dayCount = core.DayCount.act360();
   * const start = core.createDate(2025, 1, 15);
   * const end   = core.createDate(2025, 4, 15);
   * dayCount.yearFraction(start, end); // 90 / 360 = 0.25
   * ```
   * @param startEpochDays - Start date as days since 1970-01-01.
   * @param endEpochDays - End date as days since 1970-01-01.
   * @returns Year fraction (`>= 0` if `end >= start`).
   * @throws If either date is out of representable range.
   */
  yearFraction(startEpochDays: number, endEpochDays: number): number;
  /**
   * Compute a signed year fraction, preserving the start/end orientation.
   * @returns Returns the computed numeric result in the units described above.
   * @param startEpochDays - Start date as days since 1970-01-01.
   * @param endEpochDays - End date as days since 1970-01-01.
   * @throws Error - Throws a JavaScript exception if either epoch-day value is outside the representable date range or the selected convention requires calendar or coupon-frequency context. Use `yearFractionWithContext` for Bus/252 and Act/Act ISMA.
   */
  signedYearFraction(startEpochDays: number, endEpochDays: number): number;
  /**
   * Compute the year fraction with explicit convention context.
   * @returns Returns the computed numeric result in the units described above.
   * @param startEpochDays - Start date as days since 1970-01-01.
   * @param endEpochDays - End date as days since 1970-01-01.
   * @param ctx - DayCountContext supplying calendar, frequency, coupon-period, and termination metadata.
   * @throws Error - Throws a JavaScript exception if either epoch-day value is outside the representable date range, the start is after the end, the context names an unknown calendar, or the selected convention's required context is missing or invalid.
   */
  yearFractionWithContext(
    startEpochDays: number,
    endEpochDays: number,
    ctx: DayCountContext
  ): number;
  /**
   * Count the calendar days between two dates (epoch days).
   * @returns Returns the requested integer count.
   * @param startEpochDays - Start date as days since 1970-01-01.
   * @param endEpochDays - End date as days since 1970-01-01.
   * @throws Error - Throws a JavaScript exception if either epoch-day value is outside the representable date range.
   */
  calendarDays(startEpochDays: number, endEpochDays: number): bigint;
  /**
   * Convention name.
   * @returns Returns the requested string representation or JSON payload.
   */
  toString(): string;
}

/**
 * Day-count convention for computing year fractions and day counts.
 *
 * Dates are represented as **epoch days** (`i32`, days since 1970-01-01).
 * Use `createDate` to convert from a `(year, month, day)` triple.
 *
 * Available conventions and their factories:
 * - `act_360` → `DayCount.act360`
 * - `act_365f` → `DayCount.act365f`
 * - `30_360` → `DayCount.thirty360`
 * - `30e_360` → `DayCount.thirtyE360`
 * - `30e_360_isda` → `DayCount.thirtyE360Isda`
 * - `act_act` (ISDA) → `DayCount.actAct`
 * - `act_act_isma` (ICMA) → `DayCount.actActIsma`
 * - `bus_252` → `DayCount.bus252`
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const day_count = core.DayCount.act365f();
 * const start = core.createDate(2025, 1, 15);
 * const end   = core.createDate(2025, 7, 15);
 * const yf    = day_count.yearFraction(start, end);
 * // yf ≈ 0.4959 (181 / 365)
 * ```
 */
export interface DayCountConstructor {
  /**
   * Parse a day-count convention from its string name.
   *
   * @param name - Convention name (e.g. `"act_360"`, `"30_360"`, `"act_act"`).
   * @returns The parsed `DayCount`.
   * @throws If `name` is not a recognized day-count convention.
   */
  new (name: string): DayCount;
  /**
   * Create a `DayCount` value using the act360 convention.
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  act360(): DayCount;
  /**
   * Actual/365 Fixed.
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  act365f(): DayCount;
  /**
   * Actual/365L (ICMA Rule 251). Annual periods (or periods without
   * frequency context) use denominator 366 exactly when February 29 falls
   * in `(start, end]`; non-annual periods use 366 exactly when the end
   * date's year is a leap year. Otherwise the denominator is 365. This is
   * not ACT/ACT AFB.
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  act365l(): DayCount;
  /**
   * 30/360 US (Bond Basis).
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  thirty360(): DayCount;
  /**
   * 30E/360 (Eurobond Basis).
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  thirtyE360(): DayCount;
  /**
   * Create a `DayCount` value using the thirty e360 isda convention.
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  thirtyE360Isda(): DayCount;
  /**
   * Actual/Actual (ISDA).
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  actAct(): DayCount;
  /**
   * Actual/Actual (ICMA/ISMA).
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  actActIsma(): DayCount;
  /**
   * Create a `DayCount` value using the bus252 convention.
   * @returns Returns the resulting `DayCount` value or WebAssembly handle.
   */
  bus252(): DayCount;
}

/**
 * Optional context for day-count conventions that need market metadata.
 */
export interface DayCountContext extends WasmOwned {
  /**
   * Return a copy with the calendar used by Bus/252.
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   * @param calendarCode - Registered holiday-calendar identifier used by the Bus/252 convention.
   */
  withCalendar(calendarCode: string): DayCountContext;
  /**
   * Return a copy with the coupon frequency used by Act/Act ISMA.
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   * @param frequency - Coupon-frequency Tenor required by Actual/Actual ICMA calculations.
   */
  withFrequency(frequency: Tenor): DayCountContext;
  /**
   * Return a copy with the business-day basis used by Bus/252.
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   * @param busBasis - Business-day denominator for Bus/252, normally 252.
   */
  withBusBasis(busBasis: number): DayCountContext;
  /**
   * Return a copy with the reference coupon period (epoch days) used by
   * Act/Act ICMA. Errors when either date is out of range or
   * `start >= end`.
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   * @param startEpochDays - Reference coupon-period start as days since 1970-01-01.
   * @param endEpochDays - Reference coupon-period end as days since 1970-01-01.
   * @throws Error - Throws a JavaScript exception if either epoch-day value is outside the representable date range or the start is not strictly before the end.
   */
  withCouponPeriod(startEpochDays: number, endEpochDays: number): DayCountContext;
  /**
   * Return a copy indicating whether the accrual end is the instrument's
   * termination date (required by 30E/360 ISDA February-end handling).
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   * @param value - Whether the accrual end is the contractual termination date for 30E/360 ISDA.
   */
  withEndIsTerminationDate(value: boolean): DayCountContext;
}

/**
 * Optional context for day-count conventions that need market metadata.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const context = new core.DayCountContext()
 *   .withBusBasis(252)
 *   .withEndIsTerminationDate(true);
 * console.log(context);
 * ```
 */
export interface DayCountContextConstructor {
  /**
   * Create an empty day-count context.
   * @returns Returns the resulting `DayCountContext` value or WebAssembly handle.
   */
  new (): DayCountContext;
}

/**
 * A financial tenor such as `3M`, `1Y`, or `2W`.
 *
 * Tenors carry a numeric count and a unit (days, weeks, months, years).
 * Parse from strings or use the named-period factories (`Tenor.daily`,
 * `Tenor.weekly`, `Tenor.monthly`, `Tenor.quarterly`, `Tenor.semiAnnual`,
 * `Tenor.annual`).
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const t = new core.Tenor("3M");
 * t.toString();        // "3M"
 * t.toYearsSimple();   // 0.25
 *
 * const annual = core.Tenor.annual();
 * annual.toString();   // "1Y"
 * ```
 */
export interface Tenor extends WasmOwned {
  /**
   * Count exposed by this `Tenor` value.
   */
  readonly count: number;
  /**
   * Approximate length in years (simple estimate, no calendar).
   * @returns Returns the computed numeric result in the units described above.
   */
  toYearsSimple(): number;
  /**
   * Tenor string representation.
   * @returns Returns the requested string representation or JSON payload.
   */
  toString(): string;
}

/**
 * A financial tenor such as `3M`, `1Y`, or `2W`.
 *
 * Tenors carry a numeric count and a unit (days, weeks, months, years).
 * Parse from strings or use the named-period factories (`Tenor.daily`,
 * `Tenor.weekly`, `Tenor.monthly`, `Tenor.quarterly`, `Tenor.semiAnnual`,
 * `Tenor.annual`).
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const t = new core.Tenor("3M");
 * t.toString();        // "3M"
 * t.toYearsSimple();   // 0.25
 *
 * const annual = core.Tenor.annual();
 * annual.toString();   // "1Y"
 * ```
 */
export interface TenorConstructor {
  /**
   * Parse a tenor string.
   *
   * @param s - Tenor string. Accepted forms include `"3M"`, `"1Y"`,
   * @returns The parsed `Tenor`.
   * @throws If `s` cannot be parsed (unknown unit, missing count).
   */
  new (s: string): Tenor;
  /**
   * Create a `Tenor` value using the daily convention.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  daily(): Tenor;
  /**
   * Create a `Tenor` value using the weekly convention.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  weekly(): Tenor;
  /**
   * Create a `Tenor` value using the monthly convention.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  monthly(): Tenor;
  /**
   * 3-month (quarterly) tenor.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  quarterly(): Tenor;
  /**
   * 6-month (semi-annual) tenor.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  semiAnnual(): Tenor;
  /**
   * 12-month (annual) tenor.
   * @returns Returns the resulting `Tenor` value or WebAssembly handle.
   */
  annual(): Tenor;
}

/**
 * TypeScript type that constrains the accepted discount curve validation mode values.
 */
export type DiscountCurveValidationMode = 'market_standard' | 'negative_rate_friendly';

/**
 * Discount factor curve for present-value calculations.
 *
 * Built from `(time, discount_factor)` pillars where `time` is a year
 * fraction from `baseDate` and `df` is the price today of $1 paid at that
 * time. Defaults reflect the most common practitioner convention
 * (Hagan-West monotone-convex interpolation, flat-forward extrapolation,
 * Act/365 fixed day-count).
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * // OIS-style USD curve, base-date 2025-01-02, three pillars.
 * const curve = new core.DiscountCurve(
 *   "USD-OIS",
 *   "2025-01-02",
 *   [0.0, 1.0, 1.0, 0.95, 5.0, 0.78],
 *   "monotone_convex",
 *   "flat_forward",
 *   "act_365f",
 * );
 * curve.df(2.5);          // discount factor at 2.5y
 * curve.zero(2.5);        // continuously-compounded zero rate at 2.5y
 * ```
 */
export interface DiscountCurve extends WasmOwned {
  /**
   * Curve identifier.
   */
  readonly id: string;
  /**
   * Base date as ISO string.
   */
  readonly baseDate: string;
  /**
   * Discount factor at year fraction `t`.
   * @returns Returns the computed numeric result in the units described above.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   */
  df(t: number): number;
  /**
   * Continuously-compounded zero rate at year fraction `t`.
   * @returns Returns the computed numeric result in the units described above.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   */
  zero(t: number): number;
  /**
   * Continuously-compounded forward rate between `t1` and `t2`.
   * @returns Returns the computed numeric result in the units described above.
   * @param t1 - Earlier curve time in years used as the start of the forward interval.
   * @param t2 - Later curve time in years used as the end of the forward interval.
   * @throws Error - Throws a JavaScript exception if either time is non-finite, `t2` is not later than `t1`, the interval is shorter than the curve's minimum forward tenor, or either endpoint discount factor is non-finite or non-positive.
   */
  forward(t1: number, t2: number): number;
}

/**
 * Discount factor curve for present-value calculations.
 *
 * Built from `(time, discount_factor)` pillars where `time` is a year
 * fraction from `baseDate` and `df` is the price today of $1 paid at that
 * time. Defaults reflect the most common practitioner convention
 * (Hagan-West monotone-convex interpolation, flat-forward extrapolation,
 * Act/365 fixed day-count).
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * // OIS-style USD curve, base-date 2025-01-02, three pillars.
 * const curve = new core.DiscountCurve(
 *   "USD-OIS",
 *   "2025-01-02",
 *   [0.0, 1.0, 1.0, 0.95, 5.0, 0.78],
 *   "monotone_convex",
 *   "flat_forward",
 *   "act_365f",
 * );
 * curve.df(2.5);          // discount factor at 2.5y
 * curve.zero(2.5);        // continuously-compounded zero rate at 2.5y
 * ```
 */
export interface DiscountCurveConstructor {
  /**
   * Construct from an array of `[time, df]` pairs.
   *
   * @param id - Curve identifier (e.g. `"USD-OIS"`). Used as the lookup
   * @param baseDate - ISO-8601 date string (`"YYYY-MM-DD"`). All `time`
   * @param knots - Flat `[t0, df0, t1, df1, …]` array. `t` in years,
   * @param interp - Interpolation style (default `"monotone_convex"`).
   * @param extrapolation - Extrapolation policy (default
   * @param dayCount - Day-count convention (defaults to curve-ID inference).
   * @param validationMode - Rust validation preset: `"market_standard"`
   * @param forwardFloor - Required minimum implied forward when using
   * @returns The constructed `DiscountCurve`.
   * @throws If `knots` length is odd, the date is malformed, the
   */
  new (
    id: string,
    baseDate: string,
    knots: NumericArray,
    interp?: string,
    extrapolation?: string,
    dayCount?: string,
    validationMode?: DiscountCurveValidationMode,
    forwardFloor?: number | null
  ): DiscountCurve;
  /**
   * Construct a flat continuously-compounded discount curve.
   * @returns Returns the resulting `DiscountCurve` value or WebAssembly handle.
   * @param id - Stable identifier used to name and retrieve the supplied domain object.
   * @param baseDate - ISO-8601 curve base date from which time coordinates are measured.
   * @param continuousRate - Flat continuously compounded zero rate expressed as a decimal.
   * @throws Error - Throws a JavaScript exception if `baseDate` is not a valid ISO date, `continuousRate` is non-finite, or the implied discount factors are not finite and strictly positive.
   */
  flat(id: string, baseDate: string, continuousRate: number): DiscountCurve;
}

/**
 * Credit hazard-rate curve for default-probability modelling.
 *
 * Built from `(time, hazard_rate)` pillars where `time` is a year fraction
 * from `baseDate` and `hazard_rate` is the instantaneous default intensity
 * `λ(t)`. Survival is `S(t) = exp(-∫₀ᵗ λ(u) du)`.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * // Flat 200bp hazard rate, 40% recovery.
 * const hz = new core.HazardCurve(
 *   "ACME-HZD",
 *   "2025-01-02",
 *   [0.0, 0.02, 30.0, 0.02],
 *   0.4,
 * );
 * hz.sp(5.0);          // survival probability at 5y
 * hz.hazardRate(5.0);  // instantaneous hazard rate at 5y
 * ```
 */
export interface HazardCurve extends WasmOwned {
  /**
   * Curve identifier.
   */
  readonly id: string;
  /**
   * Base date as ISO string.
   */
  readonly baseDate: string;
  /**
   * Recovery rate assumed on default.
   */
  readonly recoveryRate: number;
  /**
   * Survival probability `S(t)` at year fraction `t`.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @returns The probability of surviving from the base date through `t`, in `[0, 1]`.
   */
  sp(t: number): number;
  /**
   * Instantaneous hazard rate `lambda(t)` at year fraction `t`.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @returns The annualized default intensity at `t`, expressed as a decimal rate.
   */
  hazardRate(t: number): number;
}

/**
 * Credit hazard-rate curve for default-probability modelling.
 *
 * Built from `(time, hazard_rate)` pillars where `time` is a year fraction
 * from `baseDate` and `hazard_rate` is the instantaneous default intensity
 * `λ(t)`. Survival is `S(t) = exp(-∫₀ᵗ λ(u) du)`.
 *
 * @example
 * ```javascript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * // Flat 200bp hazard rate, 40% recovery.
 * const hz = new core.HazardCurve(
 *   "ACME-HZD",
 *   "2025-01-02",
 *   [0.0, 0.02, 30.0, 0.02],
 *   0.4,
 * );
 * hz.sp(5.0);          // survival probability at 5y
 * hz.hazardRate(5.0);  // instantaneous hazard rate at 5y
 * ```
 */
export interface HazardCurveConstructor {
  /**
   * Construct from an array of `[time, hazardRate]` pairs.
   *
   * @param id - Curve identifier (e.g. `"ACME-HZD"`).
   * @param baseDate - ISO-8601 date string (`"YYYY-MM-DD"`). All `time`
   * @param knots - Flat `[t0, lambda0, t1, lambda1, …]` array. `t` in
   * @param recoveryRate - Recovery on default in `[0, 1]`. Defaults to the
   * @param dayCount - Day-count convention (default `"act_365f"`).
   * @returns The constructed `HazardCurve`.
   * @throws If `knots` length is odd, the date is malformed, the
   */
  new (
    id: string,
    baseDate: string,
    knots: NumericArray,
    recoveryRate?: number | null,
    dayCount?: string
  ): HazardCurve;
}

/**
 * Forward rate curve for a floating-rate index with a fixed tenor.
 */
export interface ForwardCurve extends WasmOwned {
  /**
   * Curve identifier.
   */
  readonly id: string;
  /**
   * Base date as ISO string.
   */
  readonly baseDate: string;
  /**
   * Contractual projection boundaries, or `null` for legacy tenor stepping.
   */
  readonly projectionGrid: Float64Array | null;
  /**
   * Business days from fixing to spot.
   */
  readonly resetLag: number;
  /**
   * Forward rate at year fraction `t`.
   * @returns Returns the computed numeric result in the units described above.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   */
  rate(t: number): number;
  /**
   * Discount-factor-implied simple forward over `(t1, t2)`.
   * @returns Returns the computed numeric result in the units described above.
   * @param t1 - Earlier curve time in years used as the start of the forward interval.
   * @param t2 - Later curve time in years used as the end of the forward interval.
   * @throws Error - Throws a JavaScript exception if either time is non-finite, `t2` is not later than `t1`, a projection discount factor cannot be computed, or the implied rate is non-finite.
   */
  rateBetween(t1: number, t2: number): number;
}

/**
 * Forward rate curve for a floating-rate index with a fixed tenor.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const curve = new core.ForwardCurve(
 *   "USD-SOFR-3M",
 *   0.25,
 *   "2026-01-02",
 *   [0, 0.03, 1, 0.035]
 * );
 * console.log(curve.rate(0.5));
 * ```
 */
export interface ForwardCurveConstructor {
  /**
   * Construct from an array of `[time, rate]` pairs.
   *
   * @returns Returns the resulting `ForwardCurve` value or WebAssembly handle.
   * @param id - Curve identifier.
   * @param tenor - Index tenor in years.
   * @param baseDate - ISO date string.
   * @param knots - Flat `[t0, rate0, t1, rate1, …]` array.
   * @param dayCount - Day-count convention (defaults to curve-ID inference).
   * @param interp - Interpolation style (default ``"linear"``).
   * @param extrapolation - Extrapolation policy (default ``"flat_forward"``).
   * @param projectionGrid - Optional contractual reset/end boundaries.
   * @param resetLag - Optional fixing-to-spot lag in business days; omit for Rust curve-ID inference.
   * @throws Error - Throws a JavaScript exception if `baseDate`, `dayCount`, `interp`, or `extrapolation` is invalid; `knots` has odd length; or canonical curve validation rejects the tenor, reset lag, knots, projection grid, or interpolation inputs.
   */
  new (
    id: string,
    tenor: number,
    baseDate: string,
    knots: NumericArray,
    dayCount?: string,
    interp?: string,
    extrapolation?: string,
    projectionGrid?: NumericArray | null,
    resetLag?: number | null
  ): ForwardCurve;
  /**
   * Construct from a named JavaScript options object.
   * @returns Returns the resulting `ForwardCurve` value or WebAssembly handle.
   * @param options - JavaScript options object defining the requested curve construction inputs.
   * @throws Error - Throws a JavaScript exception if `options` does not match the documented object shape or any contained date, convention, knot, tenor, reset-lag, projection-grid, or interpolation input fails canonical curve validation.
   */
  fromOptions(options: ForwardCurveOptions): ForwardCurve;
}

/**
 * TypeScript view of the `ForwardCurveOptions` WebAssembly value.
 */
export interface ForwardCurveOptions {
  /**
   * Stable identifier used to name or select the requested object.
   */
  id: string;
  /**
   * Tenor exposed by this `ForwardCurveOptions` value.
   */
  tenor: number;
  /**
   * ISO-8601 base or valuation date that anchors the curve time axis.
   */
  baseDate: string;
  /**
   * Knots exposed by this `ForwardCurveOptions` value.
   */
  knots: NumericArray;
  /**
   * Day-count convention used to convert dates into year fractions.
   */
  dayCount?: string;
  /**
   * Interp exposed by this `ForwardCurveOptions` value.
   */
  interp?: string;
  /**
   * Extrapolation exposed by this `ForwardCurveOptions` value.
   */
  extrapolation?: string;
  /**
   * Projection-grid specification that defines the curve's forward-rate intervals.
   */
  projectionGrid?: NumericArray | null;
  /**
   * Reset lag applied when projecting the index or forward rate.
   */
  resetLag?: number | null;
}

/**
 * SABR volatility cube for swaption pricing.
 *
 * Stores calibrated SABR parameters on an expiry × tenor grid and evaluates
 * implied volatilities via bilinear parameter interpolation followed by the
 * Hagan (2002) approximation.
 */
export interface VolCube extends WasmOwned {
  /**
   * Cube identifier.
   */
  readonly id: string;
  /**
   * Interpolation contract used across the expiry axis.
   */
  readonly interpolationMode: string;
  /**
   * Implied volatility at `(expiry, tenor, strike)`.
   *
   * Returns `Err` if `expiry` or `tenor` falls outside the grid.
   * @returns Returns the computed numeric result in the units described above.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param tenor - Underlying swap or index tenor measured in years for the quoted surface point.
   * @param strike - Option strike price in the same price units as the underlying.
   * @throws Error - Throws a JavaScript exception if `expiry` or `tenor` is outside the cube grid, `strike` is non-finite, the shifted-lognormal SABR domain is invalid, or the interpolated volatility or total variance is non-finite or non-positive.
   */
  vol(expiry: number, tenor: number, strike: number): number;
  /**
   * Implied volatility with clamped extrapolation.
   *
   * Clamps finite `expiry` and `tenor` values to the grid edges before
   * interpolation. Non-finite inputs return `NaN`.
   * @returns Returns the computed numeric result in the units described above.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param tenor - Underlying swap or index tenor measured in years for the quoted surface point.
   * @param strike - Option strike price in the same price units as the underlying.
   */
  volClamped(expiry: number, tenor: number, strike: number): number;
  /**
   * Normal (Bachelier) implied volatility at `(expiry, tenor, strike)`.
   *
   * The returned vol is in absolute rate units (e.g. `0.008` = 80 bp/yr
   * normal vol), the swaption market quoting convention.
   *
   * Returns `Err` if `expiry` or `tenor` falls outside the grid, if the
   * expansion yields a non-finite volatility, or for cross-zero quotes
   * (`(F+s)(K+s) <= 0`) with `beta > 0`, which require an explicit shift.
   * @returns Returns the computed numeric result in the units described above.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param tenor - Underlying swap or index tenor measured in years for the quoted surface point.
   * @param strike - Option strike price in the same price units as the underlying.
   * @throws Error - Throws a JavaScript exception if `expiry` or `tenor` is outside the cube grid, `strike` is non-finite, the SABR expansion is non-finite, total variance is invalid, or an unshifted positive-beta quote crosses zero.
   */
  volNormal(expiry: number, tenor: number, strike: number): number;
  /**
   * Normal (Bachelier) implied volatility with clamped extrapolation.
   *
   * Clamps finite `expiry` and `tenor` values to the grid edges; a
   * degenerate finite expansion is floored to a small positive normal vol
   * (absolute rate units). Non-finite inputs return `NaN`.
   * @returns Returns the computed numeric result in the units described above.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param tenor - Underlying swap or index tenor measured in years for the quoted surface point.
   * @param strike - Option strike price in the same price units as the underlying.
   */
  volNormalClamped(expiry: number, tenor: number, strike: number): number;
}

/**
 * SABR volatility cube for swaption pricing.
 *
 * Stores calibrated SABR parameters on an expiry × tenor grid and evaluates
 * implied volatilities via bilinear parameter interpolation followed by the
 * Hagan (2002) approximation.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const cube = new core.VolCube(
 *   "USD-SWAPTION",
 *   [1],
 *   [5],
 *   [0.02, 0.5, -0.2, 0.4, Number.NaN],
 *   [0.03]
 * );
 * console.log(cube.vol(1, 5, 0.03));
 * ```
 */
export interface VolCubeConstructor {
  /**
   * Construct a vol cube from a flat SABR parameter array.
   *
   * @returns Returns the resulting `VolCube` value or WebAssembly handle.
   * @param id - Curve identifier.
   * @param expiries - Option expiry axis in years (strictly increasing).
   * @param tenors - Swap tenor axis in years (strictly increasing).
   * @param paramsFlat - Row-major flat array of SABR parameters: `[alpha0, beta0, rho0, nu0, shift0, alpha1, …]`. Length must equal `expiries.len() * tenors.len() * 5`. Pass `NaN` for the shift element of a node to omit the shift.
   * @param forwards - Row-major forward rates, one per grid node. @param interpolationMode - Volatility-surface interpolation mode used between quoted points.
   * @throws Error - Throws a JavaScript exception if an axis is empty, non-finite, non-positive, or not strictly increasing; the parameter or forward array has the wrong length; a forward is non-finite; any SABR node has invalid alpha, beta, rho, nu, or shift; or `interpolationMode` is neither `vol` nor `total_variance`.
   */
  new (
    id: string,
    expiries: NumericArray,
    tenors: NumericArray,
    paramsFlat: NumericArray,
    forwards: NumericArray,
    interpolationMode?: string
  ): VolCube;
}

/**
 * Typed FX conversion policy wrapper for WASM callers.
 */
export interface FxConversionPolicy extends WasmOwned {
  /**
   * String form of the conversion policy.
   * @returns Returns the requested string representation or JSON payload.
   */
  toString(): string;
}

/**
 * Typed FX conversion policy wrapper for WASM callers.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const policy = core.FxConversionPolicy.cashflowDate();
 * console.log(policy.toString());
 * ```
 */
export interface FxConversionPolicyConstructor {
  /**
   * Use spot/forward on the cashflow date.
   * @returns Returns the resulting `FxConversionPolicy` value or WebAssembly handle.
   */
  cashflowDate(): FxConversionPolicy;
  /**
   * Use period end date.
   * @returns Returns the resulting `FxConversionPolicy` value or WebAssembly handle.
   */
  periodEnd(): FxConversionPolicy;
  /**
   * Use an average over the period.
   * @returns Returns the resulting `FxConversionPolicy` value or WebAssembly handle.
   */
  periodAverage(): FxConversionPolicy;
  /**
   * Use a custom provider-defined strategy.
   * @returns Returns the resulting `FxConversionPolicy` value or WebAssembly handle.
   */
  custom(): FxConversionPolicy;
  /**
   * Parse from a string label such as ``\"cashflow_date\"``.
   * @returns Returns the resulting `FxConversionPolicy` value or WebAssembly handle.
   * @param name - Name supplied to from name; follow the type and convention required by the surrounding API.
   * @throws Error - Throws a JavaScript exception unless `name` is `cashflow_date`, `period_end`, `period_average`, or `custom`.
   */
  fromName(name: string): FxConversionPolicy;
}

/**
 * Structured FX lookup result for WASM callers.
 */
export interface FxRateResult extends WasmOwned {
  /**
   * The FX conversion rate.
   */
  readonly rate: number;
  /**
   * Whether the rate was obtained via triangulation.
   */
  readonly triangulated: boolean;
}

/**
 * `FxRateResult` has no public constructor; instances come from `FxMatrix.rate`.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const matrix = new core.FxMatrix();
 * matrix.setQuote("EUR", "USD", 1.1);
 * const result = matrix.rate(
 *   "EUR",
 *   "USD",
 *   "2026-01-02",
 *   core.FxConversionPolicy.cashflowDate()
 * );
 * console.log(result.rate, result.triangulated);
 * ```
 */
export interface FxRateResultConstructor {
  /**
   * Prototype exposed by this `FxRateResult` value.
   */
  readonly prototype: FxRateResult;
}

/**
 * Foreign-exchange rate matrix for currency conversion.
 */
export interface FxMatrix extends WasmOwned {
  /**
   * Set an explicit FX quote.
   *
   * @param base - Base (from) currency ISO code.
   * @param quote - Quote (to) currency ISO code.
   * @param rate - Conversion rate.
   * @throws Error - Throws a JavaScript exception if either currency code is invalid or `rate` is non-finite or not strictly positive.
   */
  setQuote(base: string, quote: string, rate: number): void;
  /**
   * Set an authoritative quote scoped to one date and conversion policy.
   * @param base - Base currency code of the FX quote, where the rate is quote per base.
   * @param quote - Quote currency code of the FX rate, expressed per unit of base currency.
   * @param date - ISO-8601 date used by the calculation or market-data lookup.
   * @param policy - FX quote-selection policy for resolving direct, inverse, or triangulated rates.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @throws Error - Throws a JavaScript exception if either currency code is invalid, `date` is not a valid ISO date, or `rate` is non-finite or not strictly positive.
   */
  setQuoteOn(
    base: string,
    quote: string,
    date: string,
    policy: FxConversionPolicy,
    rate: number
  ): void;
  /**
   * Look up an FX rate.
   *
   * @returns Returns the resulting `FxRateResult` value or WebAssembly handle.
   * @param base - Base (from) currency ISO code.
   * @param quote - Quote (to) currency ISO code.
   * @param date - ISO date string.
   * @param policy - Reusable conversion policy handle.
   * @throws Error - Throws a JavaScript exception if either currency code or `date` is invalid, no direct, inverse, or triangulated quote is available, or a resolved quote is non-finite or non-positive.
   */
  rate(base: string, quote: string, date: string, policy: FxConversionPolicy): FxRateResult;
  /**
   * Look up an FX rate using cashflow-date conversion semantics.
   * @returns Returns the resulting `FxRateResult` value or WebAssembly handle.
   * @param base - Base currency code of the FX quote, where the rate is quote per base.
   * @param quote - Quote currency code of the FX rate, expressed per unit of base currency.
   * @param date - ISO-8601 date used by the calculation or market-data lookup.
   * @throws Error - Throws a JavaScript exception if either currency code or `date` is invalid, no direct, inverse, or triangulated cashflow-date quote is available, or a resolved quote is non-finite or non-positive.
   */
  rateDefault(base: string, quote: string, date: string): FxRateResult;
}

/**
 * Foreign-exchange rate matrix for currency conversion.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const matrix = new core.FxMatrix();
 * matrix.setQuote("EUR", "USD", 1.1);
 * console.log(matrix.rateDefault("EUR", "USD", "2026-01-02").rate);
 * ```
 */
export interface FxMatrixConstructor {
  /**
   * Create an empty FX matrix.
   * @returns Returns the resulting `FxMatrix` value or WebAssembly handle.
   */
  new (): FxMatrix;
}

/**
 * FX vol surface quoted in **delta space** (ATM, 25-delta RR/BF, optional
 * 10-delta wings).
 *
 * Stores market-standard FX delta quotes (Wystup 2006, Clark 2011) and
 * converts to a strike-axis volatility surface on demand via Garman-Kohlhagen.
 * The delta convention is **forward delta (premium-unadjusted)**.
 */
export interface FxDeltaVolSurface extends WasmOwned {
  /**
   * Surface identifier.
   */
  readonly id: string;
  /**
   * Expiry axis in years.
   */
  readonly expiries: Float64Array;
  /**
   * Number of expiry pillars.
   */
  readonly numExpiries: number;
  /**
   * Pillar vols at the given expiry index as `[atm, put25d_vol, call25d_vol]`.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param expiryIdx - Zero-based index of the requested expiry pillar in the volatility surface.
   * @throws Error - Throws a JavaScript exception if `expiryIdx` is outside the surface's expiry axis.
   */
  pillarVols(expiryIdx: number): Float64Array;
  /**
   * Implied vol at `(expiry, strike)` for the supplied forward.
   * @returns Returns the computed numeric result in the units described above.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @throws Error - Throws a JavaScript exception if `expiry`, `strike`, or `forward` is not finite and strictly positive, a quoted wing implies a non-positive volatility, or the delta-space smile cannot be constructed.
   */
  impliedVol(expiry: number, strike: number, forward: number): number;
}

/**
 * FX vol surface quoted in **delta space** (ATM, 25-delta RR/BF, optional
 * 10-delta wings).
 *
 * Stores market-standard FX delta quotes (Wystup 2006, Clark 2011) and
 * converts to a strike-axis volatility surface on demand via Garman-Kohlhagen.
 * The delta convention is **forward delta (premium-unadjusted)**.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const surface = new core.FxDeltaVolSurface(
 *   "EURUSD-VOL",
 *   [1],
 *   [0.12],
 *   [0.01],
 *   [0.002]
 * );
 * console.log(surface.pillarVols(0));
 * ```
 */
export interface FxDeltaVolSurfaceConstructor {
  /**
   * Construct an FX delta-quoted vol surface with 25-delta wings.
   *
   * Optional `rr10d` / `bf10d` add 10-delta wings for richer wing
   * interpolation. Pass an empty array for both to omit; if one is
   * provided, the other must be too.
   *
   * @returns Returns the resulting `FxDeltaVolSurface` value or WebAssembly handle.
   * @param id - Stable surface identifier.
   * @param expiries - Strictly increasing positive expiry times (years).
   * @param atmVols - ATM delta-neutral straddle vols per expiry.
   * @param rr25d - 25-delta risk reversal per expiry (call vol − put vol).
   * @param bf25d - 25-delta butterfly per expiry (wing avg − ATM).
   * @param rr10d - Optional 10-delta risk reversal per expiry.
   * @param bf10d - Optional 10-delta butterfly per expiry.
   * @throws Error - Throws a JavaScript exception if `rr10d` and `bf10d` are not both present or both absent; quote arrays are empty or have mismatched lengths; expiries are not finite, positive, and strictly increasing; ATM vols are not finite and positive; or any risk reversal or butterfly is non-finite.
   */
  new (
    id: string,
    expiries: NumericArray,
    atmVols: NumericArray,
    rr25d: NumericArray,
    bf25d: NumericArray,
    rr10d?: NumericArray,
    bf10d?: NumericArray
  ): FxDeltaVolSurface;
  /**
   * Convert a forward delta to a strike (Garman-Kohlhagen, premium-unadjusted).
   * @returns Returns the computed numeric result in the units described above.
   * @param delta - Option delta expressed under the surface's documented delta convention.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   */
  deltaToStrike(delta: number, forward: number, vol: number, expiry: number): number;
  /**
   * Convert a strike to forward delta (Garman-Kohlhagen call delta).
   * @returns Returns the computed numeric result in the units described above.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   */
  strikeToDelta(strike: number, forward: number, vol: number, expiry: number): number;
}

/**
 * Monte Carlo pricer result (JSON object from Rust).
 */
export interface MonteCarloEstimateJson {
  /**
   * Mean exposed by this `MonteCarloEstimateJson` value.
   */
  mean: number;
  /**
   * Currency exposed by this `MonteCarloEstimateJson` value.
   */
  currency: string;
  /**
   * Stderr exposed by this `MonteCarloEstimateJson` value.
   */
  stderr: number;
  /**
   * Sample standard deviation (absent when not computed).
   */
  std_dev?: number;
  /**
   * Ci lower exposed by this `MonteCarloEstimateJson` value.
   */
  ci_lower: number;
  /**
   * Ci upper exposed by this `MonteCarloEstimateJson` value.
   */
  ci_upper: number;
  /**
   * Number of independent path estimators; equals `num_simulated_paths` without variance reduction, half of it with antithetic pairing.
   */
  num_paths: number;
  /**
   * Total number of simulated sample paths; `2 * num_paths` with antithetic variates, otherwise equals `num_paths`.
   */
  num_simulated_paths: number;
  /**
   * Median of captured discounted path values (absent when paths are not captured).
   */
  median?: number;
  /**
   * 25th percentile of captured discounted path values (absent when paths are not captured).
   */
  percentile_25?: number;
  /**
   * 75th percentile of captured discounted path values (absent when paths are not captured).
   */
  percentile_75?: number;
  /**
   * Minimum of captured discounted path values (absent when paths are not captured).
   */
  min?: number;
  /**
   * Maximum of captured discounted path values (absent when paths are not captured).
   */
  max?: number;
  /**
   * Relative standard error (`stderr / |mean|`); `Infinity` near zero mean.
   */
  relative_stderr: number;
}

/**
 * Variation margin calculator result (JSON object from Rust).
 */
export interface VariationMarginJson {
  /**
   * Gross exposure exposed by this `VariationMarginJson` value.
   */
  gross_exposure: number;
  /**
   * Net exposure exposed by this `VariationMarginJson` value.
   */
  net_exposure: number;
  /**
   * Delivery amount exposed by this `VariationMarginJson` value.
   */
  delivery_amount: number;
  /**
   * Return amount exposed by this `VariationMarginJson` value.
   */
  return_amount: number;
  /**
   * Net margin exposed by this `VariationMarginJson` value.
   */
  net_margin: number;
  /**
   * Requires call exposed by this `VariationMarginJson` value.
   */
  requires_call: boolean;
}

/**
 * Bilateral XVA result (JSON object from Rust).
 *
 * Adjustments are positive when they cost the desk and compose as
 * `total_xva = cva - dva + fva + mva`. Optional funding legs are absent when
 * they were not computed.
 */
export interface XvaResultJson {
  /**
   * CVA: expected loss from counterparty default.
   */
  cva: number;
  /**
   * DVA: own-default benefit. Absent when not computed.
   */
  dva?: number;
  /**
   * FVA: net funding cost/benefit. Absent when no funding config was given.
   */
  fva?: number;
  /**
   * MVA: funding cost of posted initial margin. Absent when no `im_profile`
   * was given.
   */
  mva?: number;
  /**
   * Required all-in adjustment = `cva - dva + fva + mva`.
   */
  total_xva: number;
  /**
   * Expected positive exposure profile as `[time, value]` pairs.
   */
  epe_profile: Array<[number, number]>;
  /**
   * Expected negative exposure profile as `[time, value]` pairs.
   */
  ene_profile: Array<[number, number]>;
  /**
   * Potential future exposure profile as `[time, value]` pairs.
   */
  pfe_profile: Array<[number, number]>;
  /**
   * Maximum PFE across the profile.
   */
  max_pfe: number;
  /**
   * Effective EPE profile as `[time, value]` pairs.
   */
  effective_epe_profile: Array<[number, number]>;
  /**
   * Time-weighted average effective EPE (regulatory scalar).
   */
  effective_epe: number;
}

/**
 * Forecast backtest metrics (JSON object from Rust).
 */
export interface BacktestForecastMetricsJson {
  /**
   * Mae exposed by this `BacktestForecastMetricsJson` value.
   */
  mae: number;
  /**
   * Mape exposed by this `BacktestForecastMetricsJson` value.
   */
  mape: number;
  /**
   * Rmse exposed by this `BacktestForecastMetricsJson` value.
   */
  rmse: number;
  /**
   * N exposed by this `BacktestForecastMetricsJson` value.
   */
  n: number;
}

/**
 * Gross-leverage impact of a liability management exercise.
 * Leverage is gross debt over EBITDA, so `8.0` reads as 8.0x.
 */
export interface LmeLeverageImpact {
  /**
   * Gross debt of the target instrument before the exercise.
   */
  pre_total_debt: number;
  /**
   * Gross debt of the target instrument after the exercise.
   */
  post_total_debt: number;
  /**
   * Gross debt over EBITDA before the exercise, as a multiple.
   */
  pre_leverage: number;
  /**
   * Gross debt over EBITDA after the exercise, as a multiple.
   */
  post_leverage: number;
  /**
   * Turns of leverage removed: `pre_leverage - post_leverage`.
   */
  leverage_reduction: number;
}

/**
 * Hold-versus-tender economics of a distressed exchange offer.
 */
export interface ExchangeOfferAnalysis {
  /**
   * Canonical offer structure, echoed back from the request.
   */
  exchange_type: 'par_for_par' | 'discount' | 'uptier' | 'downtier';
  /**
   * Present value of the existing claim if it is not tendered.
   */
  old_npv: number;
  /**
   * Present value of the new instrument received on tendering.
   */
  new_npv: number;
  /**
   * Cash consent or early-tender fee.
   */
  consent_fee: number;
  /**
   * Estimated value of attached equity or warrants.
   */
  equity_sweetener_value: number;
  /**
   * Total tender consideration: `new_npv + consent_fee + equity_sweetener_value`.
   */
  tender_total: number;
  /**
   * Tender consideration less the hold-out present value.
   */
  delta_npv: number;
  /**
   * Hold-out recovery fraction that matches the tender; capped at 1.0.
   */
  breakeven_recovery: number;
  /**
   * True when `tender_total` exceeds `old_npv * 1.02`.
   */
  tender_recommended: boolean;
}

/**
 * Issuer-side economics of a liability management exercise.
 */
export interface LmeAnalysis {
  /**
   * Canonical LME structure, echoed back from the request.
   */
  lme_type: 'open_market_repurchase' | 'tender_offer' | 'amend_and_extend' | 'dropdown';
  /**
   * Cash paid by the issuer, in the caller's monetary unit.
   */
  cost: number;
  /**
   * Face amount retired; zero for structures that do not extinguish debt.
   */
  notional_reduction: number;
  /**
   * Par retired less cash paid — the discount captured by the issuer.
   */
  discount_capture: number;
  /**
   * Discount captured as a fraction of par retired; zero when no par is retired.
   */
  discount_capture_pct: number;
  /**
   * Value fraction diverted from non-participating holders; nonzero only for a dropdown.
   */
  remaining_holder_impact_pct: number;
  /**
   * Gross-leverage block, or null when no positive EBITDA was supplied.
   */
  leverage_impact: LmeLeverageImpact | null;
}

/**
 * Namespaced TypeScript entry points for core calculations and types.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * console.log(core.meanArray([1, 2, 3]));
 * ```
 */
export interface CoreNamespace {
  /**
   * Currency exposed by this `Core` value.
   */
  Currency: CurrencyConstructor;
  /**
   * Money exposed by this `Core` value.
   */
  Money: MoneyConstructor;
  /**
   * Rate exposed by this `Core` value.
   */
  Rate: RateConstructor;
  /**
   * Bps exposed by this `Core` value.
   */
  Bps: BpsConstructor;
  /**
   * Percentage exposed by this `Core` value.
   */
  Percentage: PercentageConstructor;
  /**
   * Day count exposed by this `Core` value.
   */
  DayCount: DayCountConstructor;
  /**
   * Day count context exposed by this `Core` value.
   */
  DayCountContext: DayCountContextConstructor;
  /**
   * Tenor exposed by this `Core` value.
   */
  Tenor: TenorConstructor;
  /**
   * Create a date and return it as epoch days (days since 1970-01-01).
   * @returns Returns the computed numeric result in the units described above.
   * @param year - Four-digit calendar year component of the supplied date.
   * @param month - Calendar month number from 1 through 12.
   * @param day - Calendar day number within the selected month.
   * @throws Error - Throws a JavaScript exception if `month` is outside `1..=12` or the supplied year, month, and day do not form a representable calendar date.
   */
  createDate(year: number, month: number, day: number): number;
  /**
   * Convert epoch days back to `[year, month, day]` as a JS array-compatible triple.
   * @returns Returns numeric results as a `Int32Array` in the documented order.
   * @param days - Number of days since 1970-01-01 to decompose into year, month, and day.
   * @throws Error - Throws a JavaScript exception if `days` is outside the representable date range.
   */
  dateFromEpochDays(days: number): Int32Array;
  /**
   * Adjust a date (epoch days) according to a business-day convention and calendar.
   *
   * Returns the adjusted date as epoch days.
   * @returns Returns the computed numeric result in the units described above.
   * @param epochDays - Unadjusted date as days since 1970-01-01.
   * @param convention - Business-day adjustment convention string accepted by the date API.
   * @param calendarCode - Registered holiday-calendar identifier used to find business days.
   * @throws Error - Throws a JavaScript exception if `epochDays` is outside the representable date range, `convention` is unrecognized, `calendarCode` is unknown, or adjustment cannot produce a representable business date.
   */
  adjust(epochDays: number, convention: string, calendarCode: string): number;
  /**
   * Return the list of available calendar codes.
   * @returns Returns the resulting `string[]` collection in the documented order.
   */
  availableCalendars(): string[];
  /**
   * Discount curve exposed by this `Core` value.
   */
  DiscountCurve: DiscountCurveConstructor;
  /**
   * Hazard curve exposed by this `Core` value.
   */
  HazardCurve: HazardCurveConstructor;
  /**
   * Forward curve exposed by this `Core` value.
   */
  ForwardCurve: ForwardCurveConstructor;
  /**
   * Vol cube exposed by this `Core` value.
   */
  VolCube: VolCubeConstructor;
  /**
   * Fx delta vol surface exposed by this `Core` value.
   */
  FxDeltaVolSurface: FxDeltaVolSurfaceConstructor;
  /**
   * Fx conversion policy exposed by this `Core` value.
   */
  FxConversionPolicy: FxConversionPolicyConstructor;
  /**
   * Fx rate result exposed by this `Core` value.
   */
  FxRateResult: FxRateResultConstructor;
  /**
   * Fx matrix exposed by this `Core` value.
   */
  FxMatrix: FxMatrixConstructor;
  /**
   * Evaluate the static Nelson-Siegel (1987) yield curve for one factor triple.
   *
   * This is the Diebold-Li cross-sectional equation for a single date:
   * `y(tau) = b1 + b2 * s(tau) + b3 * (s(tau) - exp(-lambda * tau))` with
   * `s(tau) = (1 - exp(-lambda * tau)) / (lambda * tau)`. Returns one yield per
   * tenor, in decimal units and in input order.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param lambda - Exponential decay parameter for tenors in years; must be finite and greater than zero (0.7308 is the years-equivalent of Diebold-Li's 0.0609 months value).
   * @param level - Nelson-Siegel beta1, the long-run level factor in decimal yield units such as 0.06 for 6%.
   * @param slope - Nelson-Siegel beta2, the slope factor (negative of the short-minus-long spread) in decimal yield units.
   * @param curvature - Nelson-Siegel beta3, the hump-shaped curvature factor in decimal yield units.
   * @param tenors - Maturities in years, each finite and non-negative; output order matches this array.
   * @throws Error - Throws a JavaScript exception if `lambda` is non-finite or non-positive, any factor loading is non-finite, or any tenor is non-finite or negative.
   */
  nelsonSiegelYields(
    lambda: number,
    level: number,
    slope: number,
    curvature: number,
    tenors: NumericArray
  ): Float64Array;
  /**
   * Apply a lower-triangular factor L to a vector z, returning `L z`.
   *
   * This is the Cholesky "apply" step that turns independent standard normals
   * into correlated normals: if `A = L L^T` and `z ~ N(0, I)`, then
   * `L z ~ N(0, A)`. Accepts L as `n * n` row-major entries; only the lower
   * triangle is read and the upper triangle is assumed zero.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param l - Lower-triangular Cholesky factor as a flat row-major array of n × n entries.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @param z - Vector of length n to transform, typically independent standard-normal draws.
   * @throws Error - Throws a JavaScript exception if `n * n` overflows, `l` does not contain exactly `n * n` entries, or `z` does not contain exactly `n` entries.
   */
  applyLowerTriangular(l: NumericArray, n: number, z: NumericArray): Float64Array;
  /**
   * Cholesky decomposition of a symmetric positive-definite matrix.
   *
   * Accepts a square matrix as a nested JS array (`number[][]`, row-major)
   * and returns the lower-triangular factor L such that A = L L^T.
   * @returns Returns the resulting `number[][]` collection in the documented order.
   * @param matrix - Square numeric matrix in the nested or row-major shape required by this callable.
   * @throws Error - Throws a JavaScript exception if `matrix` cannot be decoded as a square numeric matrix, contains a non-finite value, is singular or not positive definite, or the result cannot be converted to a JavaScript array.
   */
  choleskyDecomposition(matrix: number[][]): number[][];
  /**
   * Solve a symmetric positive-definite linear system A x = b given the
   * Cholesky factor L (where A = L L^T).
   *
   * Accepts L as `number[][]` and b as `number[]`. Returns x as `number[]`.
   * @returns Returns the resulting `number[]` collection in the documented order.
   * @param chol - Lower-triangular Cholesky factor of the coefficient matrix, in the documented matrix shape.
   * @param b - Right-hand-side vector of a linear system, aligned with the Cholesky factor dimension.
   * @throws Error - Throws a JavaScript exception if either input cannot be decoded as the documented numeric array, `chol` is not square, `b` has the wrong length, a diagonal factor is singular, or the result cannot be converted to a JavaScript array.
   */
  choleskySolve(chol: number[][], b: number[]): number[];
  /**
   * Cholesky decomposition for a flat row-major matrix.
   *
   * Accepts a `Float64Array`/`number[]` containing `n * n` row-major entries
   * and returns a flat lower-triangular factor.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param matrix - Square numeric matrix in the nested or row-major shape required by this callable.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @throws Error - Throws a JavaScript exception if `n * n` overflows, `matrix` does not contain exactly `n * n` entries, or the matrix contains a non-finite value, is singular, or is not positive definite.
   */
  choleskyDecompositionFlat(matrix: NumericArray, n: number): Float64Array;
  /**
   * Solve a symmetric positive-definite linear system from a flat Cholesky factor.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param chol - Lower-triangular Cholesky factor of the coefficient matrix, in the documented matrix shape.
   * @param b - Right-hand-side vector of a linear system, aligned with the Cholesky factor dimension.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @throws Error - Throws a JavaScript exception if `n * n` overflows, `chol` does not contain exactly `n * n` entries, `b` does not contain `n` entries, or a diagonal factor is singular.
   */
  choleskySolveFlat(chol: NumericArray, b: NumericArray, n: number): Float64Array;
  /**
   * Validate a flat row-major correlation matrix.
   *
   * This is the only correlation-matrix validator on the `core` namespace.
   * Callers pass `n * n` row-major entries plus the matrix dimension `n`.
   * @param matrix - Square numeric matrix in the nested or row-major shape required by this callable.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @throws Error - Throws a JavaScript exception if `n * n` overflows, the flat length differs from `n * n`, or the matrix is not a finite, symmetric, positive-semidefinite correlation matrix with unit diagonal and coefficients in `[-1, 1]`.
   */
  validateCorrelationMatrixFlat(matrix: NumericArray, n: number): void;
  /**
   * Arithmetic mean.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Throws a JavaScript exception if `data` cannot be decoded as a numeric array.
   */
  mean(data: number[]): number;
  /**
   * Arithmetic mean over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   */
  meanArray(data: NumericArray): number;
  /**
   * Sample variance (unbiased, n-1 denominator).
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Throws a JavaScript exception if `data` cannot be decoded as a numeric array.
   */
  variance(data: number[]): number;
  /**
   * Sample variance over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   */
  varianceArray(data: NumericArray): number;
  /**
   * Population variance (n denominator).
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Throws a JavaScript exception if `data` cannot be decoded as a numeric array.
   */
  populationVariance(data: number[]): number;
  /**
   * Population variance over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   */
  populationVarianceArray(data: NumericArray): number;
  /**
   * Pearson correlation coefficient.
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Numeric observation series aligned one-for-one with the other series.
   * @param y - Numeric observation series aligned one-for-one with the other series.
   * @throws Error - Throws a JavaScript exception if `x` or `y` cannot be decoded as a numeric array.
   */
  correlation(x: number[], y: number[]): number;
  /**
   * Pearson correlation over typed numeric arrays.
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Numeric observation series aligned one-for-one with the other series.
   * @param y - Numeric observation series aligned one-for-one with the other series.
   */
  correlationArray(x: NumericArray, y: NumericArray): number;
  /**
   * Sample covariance (unbiased, n-1 denominator).
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Numeric observation series aligned one-for-one with the other series.
   * @param y - Numeric observation series aligned one-for-one with the other series.
   * @throws Error - Throws a JavaScript exception if `x` or `y` cannot be decoded as a numeric array.
   */
  covariance(x: number[], y: number[]): number;
  /**
   * Sample covariance over typed numeric arrays.
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Numeric observation series aligned one-for-one with the other series.
   * @param y - Numeric observation series aligned one-for-one with the other series.
   */
  covarianceArray(x: NumericArray, y: NumericArray): number;
  /**
   * Empirical quantile (R-7 / NumPy default) with linear interpolation.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @param q - Quantile probability from 0 through 1 used to select the order statistic.
   * @throws Error - Throws a JavaScript exception if `data` cannot be decoded as a numeric array.
   */
  quantile(data: number[], q: number): number;
  /**
   * Empirical quantile over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @param q - Quantile probability from 0 through 1 used to select the order statistic.
   */
  quantileArray(data: NumericArray, q: number): number;
  /**
   * Standard normal CDF Φ(x).
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Real-valued input to the requested scalar mathematical function.
   */
  normCdf(x: number): number;
  /**
   * Standard normal PDF φ(x).
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Real-valued input to the requested scalar mathematical function.
   */
  normPdf(x: number): number;
  /**
   * Inverse standard normal CDF Φ⁻¹(p).
   * @returns Returns the computed numeric result in the units described above.
   * @param p - Probability input strictly between 0 and 1 for the inverse normal distribution.
   */
  standardNormalInvCdf(p: number): number;
  /**
   * Error function erf(x).
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Real-valued input to the requested scalar mathematical function.
   */
  erf(x: number): number;
  /**
   * Natural logarithm of the Gamma function ln(Γ(x)).
   * @returns Returns the computed numeric result in the units described above.
   * @param x - Real-valued input to the requested scalar mathematical function.
   */
  lnGamma(x: number): number;
  /**
   * Kahan compensated summation.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   * @throws Error - Throws a JavaScript exception if `values` cannot be decoded as a numeric array.
   */
  kahanSum(values: number[]): number;
  /**
   * Kahan compensated summation over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   */
  kahanSumArray(values: NumericArray): number;
  /**
   * Neumaier compensated summation — handles mixed-sign values.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   * @throws Error - Throws a JavaScript exception if `values` cannot be decoded as a numeric array.
   */
  neumaierSum(values: number[]): number;
  /**
   * Neumaier compensated summation over a typed numeric array.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   */
  neumaierSumArray(values: NumericArray): number;
  /**
   * Count the longest consecutive run of strictly positive values.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   * @throws Error - Throws a JavaScript exception if `values` cannot be decoded as a numeric array.
   */
  countConsecutive(values: number[]): number;
  /**
   * Count the longest consecutive run of strictly positive values in a typed array.
   * @returns Returns the computed numeric result in the units described above.
   * @param values - Numeric values in the order used by the requested numerical operation.
   */
  countConsecutiveArray(values: NumericArray): number;
  /**
   * Compare hold-versus-tender economics for a distressed exchange offer.
   * Tendering is recommended only when the total consideration exceeds the
   * hold-out present value by more than 2%.
   * @returns Returns the tender total, NPV pickup, breakeven recovery, and tender recommendation.
   * @param oldPv - Present value of the existing claim if it is not tendered, in the caller's monetary unit.
   * @param newPv - Present value of the new instrument received on tendering, in the same unit as oldPv.
   * @param consentFee - Cash consent or early-tender fee paid to participating holders, in the same unit as oldPv.
   * @param equitySweetenerValue - Estimated value of equity or warrants attached to the new instrument, in the same unit as oldPv.
   * @param exchangeType - Offer structure: par_for_par (alias par), discount, uptier, or downtier.
   * @throws Error - Throws a JavaScript exception if `exchangeType` is unrecognized, any monetary input is negative or non-finite, or the result cannot be converted to a JavaScript object.
   */
  analyzeExchangeOffer(
    oldPv: number,
    newPv: number,
    consentFee: number,
    equitySweetenerValue: number,
    exchangeType: string
  ): ExchangeOfferAnalysis;
  /**
   * Compute discount capture and leverage impact for an LME transaction.
   * @returns Returns cash cost, par retired, discount captured, remaining-holder impact, and the optional leverage block.
   * @param lmeType - Structure of the exercise: open_market (aliases open_market_repurchase, omr), tender_offer (alias tender), amend_and_extend (aliases ae, a&e), or dropdown.
   * @param notional - Outstanding face amount of the target instrument, in the caller's monetary unit; must be positive.
   * @param repurchasePricePct - Price as a fraction of par for repurchases and tenders, the extension fee for amend-and-extend, or the transferred-asset fraction for a dropdown.
   * @param optAcceptancePct - Fraction of holders participating, in [0, 1].
   * @param ebitda - EBITDA in the same unit as notional; a positive value adds the leverage_impact block, null or non-positive omits it.
   * @throws Error - Throws a JavaScript exception if `lmeType` is unrecognized, `notional` is non-positive or non-finite, `optAcceptancePct` is outside `[0, 1]`, or `repurchasePricePct` is outside the range accepted for the selected LME type: `(0, 1.5]` for repurchases and tenders, `[0, 0.1]` for amend-and-extend, and `[0, 1]` for dropdowns. It also throws if the result cannot be converted to a JavaScript object.
   */
  analyzeLme(
    lmeType: string,
    notional: number,
    repurchasePricePct: number,
    optAcceptancePct: number,
    ebitda?: number | null
  ): LmeAnalysis;
}

/**
 * Namespaced TypeScript entry point for core APIs.
 */
export declare const core: CoreNamespace;

// --- analytics ------------------------------------------------------------

/**
 * TypeScript type that constrains the accepted numeric array values.
 */
export type NumericArray = number[] | Float64Array;
/**
 * TypeScript type that constrains the accepted numeric matrix values.
 */
export type NumericMatrix = NumericArray[];

/**
 * Descriptive statistics returned by `peerStats`.
 */
export interface PeerStatsJson {
  /**
   * Count exposed by this `PeerStatsJson` value.
   */
  count: number;
  /**
   * Mean exposed by this `PeerStatsJson` value.
   */
  mean: number;
  /**
   * Median exposed by this `PeerStatsJson` value.
   */
  median: number;
  /**
   * Std dev exposed by this `PeerStatsJson` value.
   */
  std_dev: number;
  /**
   * Min exposed by this `PeerStatsJson` value.
   */
  min: number;
  /**
   * Max exposed by this `PeerStatsJson` value.
   */
  max: number;
  /**
   * Q1 exposed by this `PeerStatsJson` value.
   */
  q1: number;
  /**
   * Q3 exposed by this `PeerStatsJson` value.
   */
  q3: number;
  /**
   * Interquartile range (`q3 - q1`).
   */
  iqr: number;
}

/**
 * Single-factor OLS regression result returned by `regressionFairValue`.
 */
export interface RegressionResultJson {
  /**
   * Intercept exposed by this `RegressionResultJson` value.
   */
  intercept: number;
  /**
   * Slope exposed by this `RegressionResultJson` value.
   */
  slope: number;
  /**
   * R squared exposed by this `RegressionResultJson` value.
   */
  r_squared: number;
  /**
   * Fitted value exposed by this `RegressionResultJson` value.
   */
  fitted_value: number;
  /**
   * Residual exposed by this `RegressionResultJson` value.
   */
  residual: number;
  /**
   * N exposed by this `RegressionResultJson` value.
   */
  n: number;
}

/**
 * Per-dimension decomposition in a relative value score.
 */
export interface DimensionScoreJson {
  /**
   * Label exposed by this `DimensionScoreJson` value.
   */
  label: string;
  /**
   * Percentile exposed by this `DimensionScoreJson` value.
   */
  percentile: number;
  /**
   * Z score exposed by this `DimensionScoreJson` value.
   */
  z_score: number;
  /**
   * Regression residual exposed by this `DimensionScoreJson` value.
   */
  regression_residual: number | null;
  /**
   * R squared exposed by this `DimensionScoreJson` value.
   */
  r_squared: number | null;
  /**
   * Weight exposed by this `DimensionScoreJson` value.
   */
  weight: number;
}

/**
 * Composite relative value result returned by `scoreRelativeValue`.
 */
export interface RelativeValueResultJson {
  /**
   * Company id exposed by this `RelativeValueResultJson` value.
   */
  company_id: string;
  /**
   * Composite score exposed by this `RelativeValueResultJson` value.
   */
  composite_score: number;
  /**
   * Dimensions exposed by this `RelativeValueResultJson` value.
   */
  dimensions: DimensionScoreJson[];
  /**
   * Confidence exposed by this `RelativeValueResultJson` value.
   */
  confidence: number;
  /**
   * Peer count exposed by this `RelativeValueResultJson` value.
   */
  peer_count: number;
}

/**
 * Structured formula explanation returned by `explainFormula`.
 */
export interface FormulaExplanationJson {
  /**
   * Node id exposed by this `FormulaExplanationJson` value.
   */
  node_id: string;
  /**
   * Period id exposed by this `FormulaExplanationJson` value.
   */
  period_id: string;
  /**
   * Final value exposed by this `FormulaExplanationJson` value.
   */
  final_value: number;
  /**
   * Node type exposed by this `FormulaExplanationJson` value.
   */
  node_type: string;
  /**
   * Formula text exposed by this `FormulaExplanationJson` value.
   */
  formula_text?: string | null;
  /**
   * Breakdown exposed by this `FormulaExplanationJson` value.
   */
  breakdown: FormulaExplanationStepJson[];
}

/**
 * One component in a structured formula explanation.
 */
export interface FormulaExplanationStepJson {
  /**
   * Component exposed by this `FormulaExplanationStepJson` value.
   */
  component: string;
  /**
   * Value exposed by this `FormulaExplanationStepJson` value.
   */
  value: number;
  /**
   * Operation exposed by this `FormulaExplanationStepJson` value.
   */
  operation?: string | null;
}

/**
 * A single drawdown episode returned by `drawdownDetails`.
 */
export interface DrawdownEpisode {
  /**
   * Start exposed by this `DrawdownEpisode` value.
   */
  start: string;
  /**
   * Valley exposed by this `DrawdownEpisode` value.
   */
  valley: string;
  /**
   * End exposed by this `DrawdownEpisode` value.
   */
  end: string | null;
  /**
   * Duration days exposed by this `DrawdownEpisode` value.
   */
  duration_days: number;
  /**
   * Max drawdown exposed by this `DrawdownEpisode` value.
   */
  max_drawdown: number;
  /**
   * Near recovery threshold exposed by this `DrawdownEpisode` value.
   */
  near_recovery_threshold: number;
  /**
   * Truncated at start exposed by this `DrawdownEpisode` value.
   */
  truncated_at_start: boolean;
}

/**
 * Aggregate statistics for grouped periodic returns.
 */
export interface PeriodStats {
  /**
   * Best exposed by this `PeriodStats` value.
   */
  best: number;
  /**
   * Worst exposed by this `PeriodStats` value.
   */
  worst: number;
  /**
   * Consecutive wins exposed by this `PeriodStats` value.
   */
  consecutive_wins: number;
  /**
   * Consecutive losses exposed by this `PeriodStats` value.
   */
  consecutive_losses: number;
  /**
   * Win rate exposed by this `PeriodStats` value.
   */
  win_rate: number;
  /**
   * Avg return exposed by this `PeriodStats` value.
   */
  avg_return: number;
  /**
   * Avg win exposed by this `PeriodStats` value.
   */
  avg_win: number;
  /**
   * Avg loss exposed by this `PeriodStats` value.
   */
  avg_loss: number;
  /**
   * Payoff ratio exposed by this `PeriodStats` value.
   */
  payoff_ratio: number;
  /**
   * Profit factor exposed by this `PeriodStats` value.
   */
  profit_factor: number;
  /**
   * Cpc ratio exposed by this `PeriodStats` value.
   */
  cpc_ratio: number;
  /**
   * Kelly criterion exposed by this `PeriodStats` value.
   */
  kelly_criterion: number;
}

/**
 * Dated rolling result returned by per-ticker rolling analytics.
 *
 * Exactly one metric-named key (`sharpe`, `sortino`, `volatility`, or
 * `return`) is present, matching the method that produced the series.
 */
export interface DatedSeries {
  /**
   * Dates exposed by this `DatedSeries` value.
   */
  dates: string[];
  /**
   * Sharpe exposed by this `DatedSeries` value.
   */
  sharpe?: Float64Array;
  /**
   * Sortino exposed by this `DatedSeries` value.
   */
  sortino?: Float64Array;
  /**
   * Volatility exposed by this `DatedSeries` value.
   */
  volatility?: Float64Array;
  /**
   * Return exposed by this `DatedSeries` value.
   */
  return?: Float64Array;
}

/**
 * Per-asset skewness/kurtosis pair returned by `skewKurt`.
 */
export interface SkewKurtResult {
  /**
   * Skewness exposed by this `SkewKurtResult` value.
   */
  skewness: Float64Array;
  /**
   * Kurtosis exposed by this `SkewKurtResult` value.
   */
  kurtosis: Float64Array;
}

/**
 * Per-asset VaR/ES pair returned by `valueAtRiskAndEs`.
 */
export interface VarEsResult {
  /**
   * Value at risk exposed by this `VarEsResult` value.
   */
  value_at_risk: Float64Array;
  /**
   * Expected shortfall exposed by this `VarEsResult` value.
   */
  expected_shortfall: Float64Array;
}

/**
 * OLS beta result with standard error and 95% confidence interval.
 *
 * The interval uses Student-t critical values for finite samples and an
 * asymptotic normal approximation once n - 2 >= 240.
 */
export interface BetaResult {
  /**
   * Beta exposed by this `BetaResult` value.
   */
  beta: number;
  /**
   * Std err exposed by this `BetaResult` value.
   */
  std_err: number;
  /**
   * Ci lower exposed by this `BetaResult` value.
   */
  ci_lower: number;
  /**
   * Ci upper exposed by this `BetaResult` value.
   */
  ci_upper: number;
}

/**
 * Single-factor greeks (annualized Jensen alpha, beta, R², adjusted R²).
 */
export interface GreeksResult {
  /**
   * Alpha exposed by this `GreeksResult` value.
   */
  alpha: number;
  /**
   * Beta exposed by this `GreeksResult` value.
   */
  beta: number;
  /**
   * R squared exposed by this `GreeksResult` value.
   */
  r_squared: number;
  /**
   * Adjusted r squared exposed by this `GreeksResult` value.
   */
  adjusted_r_squared: number;
}

/**
 * Rolling greeks output aligned with rolling-window end dates.
 */
export interface RollingGreeksResult {
  /**
   * Dates exposed by this `RollingGreeksResult` value.
   */
  dates: string[];
  /**
   * Alphas exposed by this `RollingGreeksResult` value.
   */
  alphas: Float64Array;
  /**
   * Betas exposed by this `RollingGreeksResult` value.
   */
  betas: Float64Array;
}

/**
 * Multi-factor regression result. Alpha is the raw regression intercept, annualized.
 */
export interface MultiFactorResult {
  /**
   * Alpha exposed by this `MultiFactorResult` value.
   */
  alpha: number;
  /**
   * Betas exposed by this `MultiFactorResult` value.
   */
  betas: number[];
  /**
   * R squared exposed by this `MultiFactorResult` value.
   */
  r_squared: number;
  /**
   * Adjusted r squared exposed by this `MultiFactorResult` value.
   */
  adjusted_r_squared: number;
  /**
   * Residual vol exposed by this `MultiFactorResult` value.
   */
  residual_vol: number;
}

/**
 * Period-to-date lookback returns (per ticker) returned by `lookbackReturns`.
 */
export interface LookbackReturns {
  /**
   * Mtd exposed by this `LookbackReturns` value.
   */
  mtd: number[];
  /**
   * Qtd exposed by this `LookbackReturns` value.
   */
  qtd: number[];
  /**
   * Ytd exposed by this `LookbackReturns` value.
   */
  ytd: number[];
  /**
   * Fytd exposed by this `LookbackReturns` value.
   */
  fytd: number[] | null;
}

/**
 * Stateful performance analytics engine over a panel of ticker series.
 *
 * `Performance` is the single entry point exposed to JS. Construct from
 * a price matrix (`new Performance(...)`) or a return matrix
 * (`Performance.fromReturns(...)`); every metric is then reachable as
 * an instance method.
 *
 * All multi-ticker scalar outputs come back as `number[]` indexed by the
 * panel's ticker order; vector / per-ticker / structured outputs are
 * serialized to plain JS objects (e.g. `DatedSeries`, `BetaResult[]`).
 */
export declare class Performance {
  constructor(
    dates: string[],
    prices: NumericMatrix,
    tickerNames: string[],
    benchmarkTicker?: string | null,
    frequency?: string
  );
  /** Construct from a return matrix (one row per `dates` entry per ticker). */
  static fromReturns(
    dates: string[],
    returns: NumericMatrix,
    tickerNames: string[],
    benchmarkTicker?: string | null,
    frequency?: string
  ): Performance;
  resetDateRange(start: string, end: string): void;
  resetBenchTicker(ticker: string): void;
  tickerNames(): string[];
  benchmarkIdx(): number;
  frequency(): string;
  /** Full return-aligned date grid as ISO date strings (independent of any active window). */
  dates(): string[];
  /** Dates of the currently active analysis window as ISO date strings. */
  activeDates(): string[];
  /** Dates for one ticker's active return series as ISO date strings. */
  activeDatesForTicker(tickerIdx: number): string[];
  cagr(): Float64Array;
  meanReturn(annualize?: boolean): Float64Array;
  volatility(annualize?: boolean): Float64Array;
  sharpe(riskFreeRate?: number): Float64Array;
  /** Sortino ratio; mar is a per-period threshold. */
  sortino(mar?: number): Float64Array;
  calmar(): Float64Array;
  maxDrawdown(): Float64Array;
  meanDrawdown(): Float64Array;
  valueAtRisk(confidence?: number): Float64Array;
  expectedShortfall(confidence?: number): Float64Array;
  trackingError(): Float64Array;
  informationRatio(): Float64Array;
  skewness(): Float64Array;
  kurtosis(): Float64Array;
  geometricMean(): Float64Array;
  /** Skewness and kurtosis from one moments pass per asset. */
  skewKurt(): SkewKurtResult;
  /** Historical VaR and expected shortfall from one tail pass per asset. */
  valueAtRiskAndEs(confidence?: number): VarEsResult;
  /** Downside deviation; mar is a per-period threshold. */
  downsideDeviation(mar?: number): Float64Array;
  maxDrawdownDuration(): number[];
  /** Empyrical-style annualized geometric up-capture. */
  upCapture(): Float64Array;
  /** Empyrical-style annualized geometric down-capture. */
  downCapture(): Float64Array;
  /** Empyrical-style annualized geometric up/down capture ratio. */
  captureRatio(): Float64Array;
  omegaRatio(threshold?: number): Float64Array;
  treynor(riskFreeRate?: number): Float64Array;
  gainToPain(): Float64Array;
  ulcerIndex(): Float64Array;
  martinRatio(): Float64Array;
  recoveryFactor(): Float64Array;
  painIndex(): Float64Array;
  painRatio(riskFreeRate?: number): Float64Array;
  tailRatio(confidence?: number): Float64Array;
  rSquared(): Float64Array;
  battingAverage(): Float64Array;
  parametricVar(confidence?: number): Float64Array;
  cornishFisherVar(confidence?: number): Float64Array;
  cdar(confidence?: number): Float64Array;
  mSquared(riskFreeRate?: number): Float64Array;
  modifiedSharpe(riskFreeRate?: number, confidence?: number): Float64Array;
  sterlingRatio(riskFreeRate?: number, n?: number): Float64Array;
  burkeRatio(riskFreeRate?: number, n?: number): Float64Array;
  /**
   * Per-period simple returns per asset, as decimal fractions (0.01 = +1%).
   *
   * Canonical accessor for the raw return panel over the active window; prefer
   * it over `excessReturns` with an all-zero risk-free series or un-compounding
   * `cumulativeReturns`. Series are span-aware and therefore ragged across
   * assets on edge-ragged panels.
   * @returns One Float64Array per asset, in `tickerNames()` order.
   */
  returns(): Float64Array[];
  /**
   * Per-period simple returns for one asset, as decimal fractions (0.01 = +1%).
   * @param tickerIdx - Zero-based ticker column index in `tickerNames()` order.
   * @returns The asset's simple return series in date order.
   * @throws If `tickerIdx` is outside the loaded ticker columns.
   */
  returnsForTicker(tickerIdx: number): Float64Array;
  cumulativeReturns(): Float64Array[];
  drawdownSeries(): Float64Array[];
  correlationMatrix(): Float64Array[];
  cumulativeReturnsOutperformance(): Float64Array[];
  drawdownDifference(): Float64Array[];
  excessReturns(rf: NumericArray, nperiods?: number): Float64Array[];
  beta(): BetaResult[];
  greeks(riskFreeRate?: number): GreeksResult[];
  rollingGreeks(tickerIdx: number, window?: number, riskFreeRate?: number): RollingGreeksResult;
  rollingVolatility(tickerIdx: number, window?: number): DatedSeries;
  rollingSortino(tickerIdx: number, window?: number, mar?: number): DatedSeries;
  rollingSharpe(tickerIdx: number, window?: number, riskFreeRate?: number): DatedSeries;
  rollingReturns(tickerIdx: number, window: number): DatedSeries;
  drawdownDetails(tickerIdx: number, n?: number): DrawdownEpisode[];
  multiFactorGreeks(tickerIdx: number, factorReturns: NumericMatrix): MultiFactorResult;
  /**
   * Period-to-date lookback returns. The FYTD window starts at the fiscal-year
   * start adjusted to the next business day on `calendar` (default `"nyse"`);
   * pass the calendar id matching your market for non-US panels.
   */
  lookbackReturns(
    refDate: string,
    fiscalYearStartMonth?: number,
    fiscalYearStartDay?: number,
    calendar?: string
  ): LookbackReturns;
  periodStats(
    tickerIdx: number,
    aggregationFrequency?: string,
    fiscalYearStartMonth?: number,
    fiscalYearStartDay?: number
  ): PeriodStats;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Namespaced TypeScript entry points for analytics calculations and types.
 * @example
 * ```typescript
 * import init, { analytics } from "finstack-quant-wasm";
 * await init();
 * const factorReturns = analytics.constrainedLeastSquares(
 *   [1, 2],
 *   1,
 *   [0.01, 0.02],
 *   [0.5, 0.5]
 * );
 * console.log(factorReturns[0]);
 * ```
 */
export interface AnalyticsNamespace {
  /**
   * `Performance` is the single entry point for analytics on a panel of
   * ticker series. Construct from prices (`new Performance(...)`) or from
   * returns (`Performance.fromReturns(...)`); every metric — return/risk
   * scalars, drawdown statistics, rolling windows, periodic returns
   * (MTD/QTD/YTD/FYTD), benchmark alpha/beta, basic factor models — is a
   * method on the resulting instance.
   */
  Performance: typeof Performance;
  /**
   * Fit factor returns satisfying the equality constraint `w'Xf = w'r`.
   *
   * Binds Rust `constrained_least_squares` (Jeet & Partani 2023, Appendix
   * A): adds the minimal Lagrangian correction to an unconstrained OLS fit
   * so the corrected factor returns exactly reproduce the weighted
   * realized return `w'r`. Typically used to fit the benchmark factor
   * returns consumed by `portfolio.factorBrinsonAttribution`, which
   * requires factor returns satisfying that same completeness condition.
   * @param exposures - Row-major factor exposure matrix, `n_assets x n_factors`: asset i's exposure to factor j is `exposures[i * n_factors + j]`.
   * @param nFactors - Number of factor columns in `exposures`; must be a positive integer no greater than `4294967295`.
   * @param returns - Realized asset returns, length `n_assets` (defines `n_assets`).
   * @param weights - Holding weights whose weighted return `w'r` must be fully reproduced by `w'Xf` (e.g. benchmark weights for a benchmark-return attribution).
   * @returns Constrained factor returns `f`, one per factor, satisfying `w'Xf = w'r` to numerical precision.
   * @throws Error - If `nFactors` is non-finite, fractional, zero, negative, or exceeds the WebAssembly `usize` range; if vector dimensions are inconsistent (including an overflowing `n_assets * n_factors`); if any vector value is non-finite; if the design matrix is rank-deficient; if coefficient rescaling or the constraint correction produces a non-finite value; or if the correction direction is degenerate and OLS does not already satisfy the constraint.
   */
  constrainedLeastSquares(
    exposures: NumericArray,
    nFactors: number,
    returns: NumericArray,
    weights: NumericArray
  ): Float64Array;
}

/**
 * Namespaced TypeScript entry point for analytics APIs.
 */
export declare const analytics: AnalyticsNamespace;

// --- factor_model.credit ------------------------------------------------------

/**
 * Calibrated credit factor hierarchy artifact.
 *
 * Produced by `CreditCalibrator` or deserialized from JSON via `fromJson`.
 * Immutable once constructed.
 */
export declare class CreditFactorModel {
  private constructor();
  /** Deserialize and validate a `CreditFactorModel` from JSON. */
  static fromJson(s: string): CreditFactorModel;
  /** Serialize to pretty-printed JSON. */
  toJson(): string;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Deterministic calibrator that produces a `CreditFactorModel`.
 *
 * Configuration and inputs are passed as JSON strings.
 */
export declare class CreditCalibrator {
  /** Construct a calibrator from a JSON-serialized `CreditCalibrationConfig`. */
  constructor(configJson: string);
  /** Run the calibration pipeline and return a `CreditFactorModel`. */
  calibrate(inputsJson: string): CreditFactorModel;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Snapshot of all hierarchy-level factor values at a single date.
 *
 * Produced by `decomposeLevels`. Pass to `decomposePeriod` to compute
 * period-over-period changes.
 */
export declare class LevelsAtDate {
  private constructor();
  /** Serialize the snapshot to pretty-printed JSON. */
  toJson(): string;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Component-wise difference between two `LevelsAtDate` snapshots.
 *
 * Produced by `decomposePeriod`.
 */
export declare class PeriodDecomposition {
  private constructor();
  /** Serialize the decomposition to pretty-printed JSON. */
  toJson(): string;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Vol-forecast view over a calibrated `CreditFactorModel`.
 *
 * `VolHorizon::Custom` is intentionally **not** exposed.
 *
 * Horizon strings accepted by `covarianceAt`, `idiosyncraticVol`, and
 * `factorModelAt`:
 * - `"one_step"` — calibrated annualized variance unchanged.
 * - `"unconditional"` — long-run.
 * - `'{"n_steps": N}'` — variance scaled by `N`.
 */
export declare class FactorCovarianceForecast {
  constructor(model: CreditFactorModel);
  /**
   * Build the factor covariance matrix at the requested horizon.
   * Returns pretty-printed JSON of a `FactorCovarianceMatrix`.
   */
  covarianceAt(horizonJson: string): string;
  /** Idiosyncratic vol (std dev) for a specific issuer at the requested horizon. */
  idiosyncraticVol(issuerId: string, horizonJson: string): number;
  /**
   * Build a portfolio-level `FactorModelConfig` JSON at the given horizon and
   * risk measure.
   */
  factorModelAt(horizonJson: string, riskMeasureJson: string): string;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Decompose observed issuer spreads at a point in time into per-level factor
 * values and per-issuer residual adders.
 *
 * @example
 * ```typescript
 * import init, {
 *   decomposeLevels,
 *   type CreditFactorModel,
 *   type LevelsAtDate
 * } from "finstack-quant-wasm";
 * await init();
 * function currentLevels(model: CreditFactorModel): LevelsAtDate {
 *   return decomposeLevels(model, '{"ACME": 0.012}', 0.01, "2026-01-02");
 * }
 * ```
 * @returns Returns the resulting `LevelsAtDate` value or WebAssembly handle.
 * @param model - Calibrated CreditFactorModel used to produce the covariance forecast.
 * @param observedSpreadsJson - JSON-serialized observed credit spreads used in the level decomposition.
 * @param observedGeneric - Observed generic-market spread component aligned with the model factors.
 * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
 * @param runtimeTagsJson - Optional runtime-tag JSON selecting the active factor-model configuration.
 * @throws Error - Throws if an issuer has no model row and no `runtime_tags` entry, or if `as_of` cannot be parsed.
 */
export declare function decomposeLevels(
  model: CreditFactorModel,
  observedSpreadsJson: string,
  observedGeneric: number,
  asOf: string,
  runtimeTagsJson?: string
): LevelsAtDate;

/**
 * Difference two `LevelsAtDate` snapshots component-wise.
 *
 * Output is restricted to buckets and issuers present in **both** snapshots.
 * @example
 * ```typescript
 * import init, {
 *   decomposePeriod,
 *   type LevelsAtDate,
 *   type PeriodDecomposition
 * } from "finstack-quant-wasm";
 * await init();
 * function periodChange(
 *   fromLevels: LevelsAtDate,
 *   toLevels: LevelsAtDate
 * ): PeriodDecomposition {
 *   return decomposePeriod(fromLevels, toLevels);
 * }
 * ```
 * @returns Returns the resulting `PeriodDecomposition` value or WebAssembly handle.
 * @param fromLevels - Credit-factor levels at the start of the attribution period.
 * @param toLevels - Credit-factor levels at the end of the attribution period.
 * @throws Error - Throws if `from_levels.date > to_levels.date` or the snapshots disagree on hierarchy depth.
 */
export declare function decomposePeriod(
  fromLevels: LevelsAtDate,
  toLevels: LevelsAtDate
): PeriodDecomposition;

/**
 * Namespaced TypeScript entry points for factor model credit calculations and types.
 * @example
 * ```typescript
 * import init, { factor_model } from "finstack-quant-wasm";
 * await init();
 * const config = JSON.stringify({
 *   policy: "globally_off",
 *   hierarchy: { levels: [] },
 *   min_bucket_size_per_level: { per_level: [] },
 *   vol_model: "sample",
 *   covariance_strategy: "diagonal",
 *   beta_shrinkage: "none",
 *   use_returns_or_levels: "returns",
 *   annualization_factor: 12
 * });
 * const calibrator = new factor_model.credit.CreditCalibrator(config);
 * calibrator.free();
 * ```
 */
export interface FactorModelCreditNamespace {
  /**
   * Credit factor model exposed by this `FactorModelCredit` value.
   */
  CreditFactorModel: typeof CreditFactorModel;
  /**
   * Credit calibrator exposed by this `FactorModelCredit` value.
   */
  CreditCalibrator: typeof CreditCalibrator;
  /**
   * Levels at date exposed by this `FactorModelCredit` value.
   */
  LevelsAtDate: typeof LevelsAtDate;
  /**
   * Period decomposition exposed by this `FactorModelCredit` value.
   */
  PeriodDecomposition: typeof PeriodDecomposition;
  /**
   * Factor covariance forecast exposed by this `FactorModelCredit` value.
   */
  FactorCovarianceForecast: typeof FactorCovarianceForecast;
  /**
   * Decompose observed issuer spreads at a point in time into per-level factor
   * values and per-issuer residual adders.
   *
   * - `model` — calibrated `CreditFactorModel`.
   * - `observed_spreads_json` — JSON `{issuer_id: spread}` map.
   * - `observed_generic` — generic (PC) factor value at `as_of`.
   * - `as_of` — ISO 8601 date string.
   * - `runtime_tags_json` — optional JSON `{issuer_id: {dim_key: tag}}` for
   *   issuers not present in the model artifact.
   *
   * Returns a `LevelsAtDate` handle.
   *
   * # Errors
   * Throws if an issuer has no model row and no `runtime_tags` entry, or if
   * `as_of` cannot be parsed.
   * @returns Returns the resulting `LevelsAtDate` value or WebAssembly handle.
   * @param model - Calibrated CreditFactorModel used to produce the covariance forecast.
   * @param observedSpreadsJson - JSON-serialized observed credit spreads used in the level decomposition.
   * @param observedGeneric - Observed generic-market spread component aligned with the model factors.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param runtimeTagsJson - Optional runtime-tag JSON selecting the active factor-model configuration.
   * @throws Error - Throws if an issuer has no model row and no `runtime_tags` entry, or if `as_of` cannot be parsed.
   */
  decomposeLevels(
    model: CreditFactorModel,
    observedSpreadsJson: string,
    observedGeneric: number,
    asOf: string,
    runtimeTagsJson?: string
  ): LevelsAtDate;
  /**
   * Difference two `LevelsAtDate` snapshots component-wise.
   *
   * Output buckets and issuers are restricted to those present in **both**
   * snapshots so the linear reconciliation invariant on `ΔS_i` holds.
   *
   * # Errors
   * Throws if `from_levels.date > to_levels.date` or the snapshots disagree
   * on hierarchy depth.
   * @returns Returns the resulting `PeriodDecomposition` value or WebAssembly handle.
   * @param fromLevels - Credit-factor levels at the start of the attribution period.
   * @param toLevels - Credit-factor levels at the end of the attribution period.
   * @throws Error - Throws if `from_levels.date > to_levels.date` or the snapshots disagree on hierarchy depth.
   */
  decomposePeriod(fromLevels: LevelsAtDate, toLevels: LevelsAtDate): PeriodDecomposition;
}

/**
 * Namespaced TypeScript entry points for factor model calculations and types.
 * @example
 * ```typescript
 * import init, { factor_model } from "finstack-quant-wasm";
 * await init();
 * const credit = factor_model.credit;
 * const config = JSON.stringify({
 *   policy: "globally_off",
 *   hierarchy: { levels: [] },
 *   min_bucket_size_per_level: { per_level: [] },
 *   vol_model: "sample",
 *   covariance_strategy: "diagonal",
 *   beta_shrinkage: "none",
 *   use_returns_or_levels: "returns",
 *   annualization_factor: 12
 * });
 * const calibrator = new credit.CreditCalibrator(config);
 * calibrator.free();
 * ```
 */
export interface FactorModelNamespace {
  /**
   * Credit factor hierarchy artifacts, calibration, and decomposition.
   */
  credit: FactorModelCreditNamespace;
}

/**
 * Namespaced TypeScript entry point for factor model APIs.
 */
export declare const factor_model: FactorModelNamespace;

// --- features ---------------------------------------------------------------

/**
 * TypeScript type that constrains the accepted feature value values.
 */
export type FeatureValue = number | null;
/**
 * TypeScript type that constrains the accepted feature params values.
 */
export type FeatureParams = Record<string, unknown>;

/**
 * Vectorized panel feature transforms.
 *
 * `values` accepts finite numbers or `null`; non-finite values are treated as
 * missing by the Rust crate. Time-series transforms are grouped by `entity` and
 * sorted by `order`; cross-sectional transforms partition by `timeKey`.
 * @example
 * ```typescript
 * import init, { features } from "finstack-quant-wasm";
 * await init();
 * const changes = features.transformTimeseries(
 *   [1, 3, 6],
 *   ["ACME", "ACME", "ACME"],
 *   ["2026-01-01", "2026-01-02", "2026-01-03"],
 *   "diff"
 * );
 * console.log(changes);
 * ```
 */
export interface FeaturesNamespace {
  /**
   * Transform a time-series panel column per entity.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param entity - Entity identifier used to group ordered time-series observations.
   * @param order - Observation-order key used to sort each entity time series.
   * @param op - Transformation operation identifier supported by the feature-engineering API.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal row counts, an unsupported `op`, malformed operation parameters, or a result that cannot be serialized to JavaScript.
   */
  transformTimeseries(
    values: FeatureValue[],
    entity: string[],
    order: string[],
    op: string,
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Transform a cross-section per timestamp.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param op - Transformation operation identifier supported by the feature-engineering API.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal `values` and `time_key` lengths, an unsupported `op`, malformed operation parameters, or a result that cannot be serialized to JavaScript.
   */
  transformCrossSectional(
    values: FeatureValue[],
    timeKey: string[],
    op: string,
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Transform a cross-section within each time/group sub-partition.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param groups - Group labels aligned with values for within-group cross-sectional operations.
   * @param op - Transformation operation identifier supported by the feature-engineering API.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal `values`, `time_key`, and `groups` lengths, an unsupported `op`, malformed operation parameters, or a result that cannot be serialized to JavaScript.
   */
  transformCrossSectionalGrouped(
    values: FeatureValue[],
    timeKey: string[],
    groups: string[],
    op: string,
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Remove cross-sectional exposure effects by OLS residualization.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param exposures - Factor-exposure matrix aligned with the supplied observations.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal row counts, exposure columns whose lengths differ from `values`, a non-boolean `fit_intercept`, or a result that cannot be serialized to JavaScript.
   */
  neutralize(
    values: FeatureValue[],
    timeKey: string[],
    exposures: FeatureValue[][],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Transform two time-series panel columns per entity.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param other - Second value series aligned with the primary series for a pairwise transformation.
   * @param entity - Entity identifier used to group ordered time-series observations.
   * @param order - Observation-order key used to sort each entity time series.
   * @param op - Transformation operation identifier supported by the feature-engineering API.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal row counts, an unsupported `op`, non-positive or non-integer `window` or `min_periods` parameters, or a result that cannot be serialized to JavaScript.
   */
  transformTimeseriesPairwise(
    values: FeatureValue[],
    other: FeatureValue[],
    entity: string[],
    order: string[],
    op: string,
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Return rolling OLS residuals per entity.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param exposures - Factor-exposure matrix aligned with the supplied observations.
   * @param entity - Entity identifier used to group ordered time-series observations.
   * @param order - Observation-order key used to sort each entity time series.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal row counts, exposure columns whose lengths differ from `values`, malformed `window`, `min_periods`, or `fit_intercept` parameters, or a result that cannot be serialized to JavaScript.
   */
  rollingRegressionResidual(
    values: FeatureValue[],
    exposures: FeatureValue[][],
    entity: string[],
    order: string[],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Convert a signal to inverse-risk-scaled weights per timestamp.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric signal observations aligned with `timeKey` and `volatility`.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param volatility - Row-aligned risk estimates used as `signal / volatility`; zero, missing, or non-finite values yield missing weights.
   * @throws Error - Rejects inputs that cannot be decoded into the declared arrays, unequal `values`, `time_key`, and `volatility` lengths, or a result that cannot be serialized to JavaScript.
   * @param params - Params input consumed by this operation.
   */
  riskScaledWeights(
    values: FeatureValue[],
    timeKey: string[],
    volatility: FeatureValue[],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Apply the default signal cleaning pass.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal `values` and `time_key` lengths, malformed clipping bounds, or a result that cannot be serialized to JavaScript.
   */
  cleanSignal(
    values: FeatureValue[],
    timeKey: string[],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Normalize a signal cross-sectionally.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal `values` and `time_key` lengths, a non-string or unsupported normalization method, malformed operation parameters, or a result that cannot be serialized to JavaScript.
   */
  normalizeSignal(
    values: FeatureValue[],
    timeKey: string[],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Convert ranks into long/short weights.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @throws Error - Rejects inputs that cannot be decoded into the declared arrays, unequal `values` and `time_key` lengths, or a result that cannot be serialized to JavaScript.
   * @param params - Params input consumed by this operation.
   */
  rankToWeights(
    values: FeatureValue[],
    timeKey: string[],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Neutralize a signal and z-score residuals.
   * @returns Returns the resulting `FeatureValue[]` collection in the documented order.
   * @param values - Numeric observations in the shape and order required by the selected transformation.
   * @param timeKey - Cross-sectional time key shared by values evaluated in the same slice.
   * @param exposures - Factor-exposure matrix aligned with the supplied observations.
   * @param params - Operation-specific parameter object defining transformation settings.
   * @throws Error - Rejects values that cannot be decoded into the declared arrays or JSON parameters, unequal row counts, exposure columns whose lengths differ from `values`, a non-boolean `fit_intercept`, or a result that cannot be serialized to JavaScript.
   */
  neutralizeAndZscore(
    values: FeatureValue[],
    timeKey: string[],
    exposures: FeatureValue[][],
    params?: FeatureParams | null
  ): FeatureValue[];
  /**
   * Apply a JSON panel transform pipeline.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical panel-transformation JSON specifying input columns, operations, and parameters.
   * @throws Error - Rejects malformed JSON or panel specifications, blank or duplicate operation names, missing partition columns, unequal row counts, malformed operation parameters, operations that cannot be evaluated, or a result that cannot be serialized to JSON.
   */
  transformPanel(specJson: string): string;
}

/**
 * Namespaced TypeScript entry point for features APIs.
 */
export declare const features: FeaturesNamespace;

// --- valuations.correlation -------------------------------------------------

/**
 * Concrete copula model for portfolio default correlation.
 */
export interface Copula extends WasmOwned {
  /**
   * Number of systematic factors in the model.
   */
  readonly numFactors: number;
  /**
   * Model name for diagnostics.
   */
  readonly modelName: string;
  /**
   * Conditional default probability given factor realization(s).
   * @returns Returns the computed numeric result in the units described above.
   * @param defaultThreshold - Latent-variable default threshold corresponding to the marginal default probability.
   * @param factorRealization - Realized systematic-factor value conditioning the default probability.
   * @param correlation - Dependence correlation from -1 through 1 under the selected copula or recovery model.
   * @throws Error - Throws a JavaScript exception if the factor count does not match the copula, any input is non-finite, `correlation` is outside `[0, 1]`, or the model produces a probability outside `[0, 1]`.
   */
  conditionalDefaultProb(
    defaultThreshold: number,
    factorRealization: number[],
    correlation: number
  ): number;
  /**
   * Strict lower-tail dependence coefficient `λ_L` at the given
   * correlation.
   *
   * Returns `NaN` when the model has no closed-form `λ_L` (Random Factor
   * Loading); check `Number.isNaN()` before using the result. For the
   * RFL heuristic stress gauge use `stressCorrelationProxy` instead.
   * @returns Returns the computed numeric result in the units described above.
   * @param correlation - Dependence correlation from -1 through 1 under the selected copula or recovery model.
   */
  tailDependence(correlation: number): number;
  /**
   * Heuristic stress-correlation proxy for the Random Factor Loading
   * copula.
   *
   * This is **not** the strict copula lower-tail-dependence coefficient
   * `λ_L` (which has no closed form for RFL — `tailDependence` returns
   * `NaN`). It gauges the extra correlation mass in the high-loading
   * tail and vanishes in the Gaussian (`loadingVol = 0`) limit.
   *
   * Throws for non-RFL copulas.
   * @returns Returns the computed numeric result in the units described above.
   * @param correlation - Dependence correlation from -1 through 1 under the selected copula or recovery model.
   * @throws Error - Throws a JavaScript exception if this copula is not a Random Factor Loading model.
   */
  stressCorrelationProxy(correlation: number): number;
}

/**
 * Copula model specification for configuration and deferred construction.
 */
export interface CopulaSpec extends WasmOwned {
  /**
   * True if this is a Gaussian spec.
   */
  readonly isGaussian: boolean;
  /**
   * True if this is a Student-t spec.
   */
  readonly isStudentT: boolean;
  /**
   * True if this is a Random Factor Loading spec.
   */
  readonly isRfl: boolean;
  /**
   * True if this is a Multi-factor spec.
   */
  readonly isMultiFactor: boolean;
  /**
   * Build a concrete copula from this specification.
   * @returns Returns the resulting `Copula` value or WebAssembly handle.
   * @throws Error - Throws a JavaScript exception if a Student-t specification contains non-finite degrees of freedom or a value at most two.
   */
  build(): Copula;
}

/**
 * Copula model specification for configuration and deferred construction.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const copula = valuations.correlation.CopulaSpec.gaussian().build();
 * console.log(copula.modelName, copula.numFactors);
 * ```
 */
export interface CopulaSpecConstructor {
  /**
   * One-factor Gaussian copula (market standard).
   * @returns Returns the resulting `CopulaSpec` value or WebAssembly handle.
   */
  gaussian(): CopulaSpec;
  /**
   * Student-t copula with specified degrees of freedom (must be > 2).
   * @returns Returns the resulting `CopulaSpec` value or WebAssembly handle.
   * @param df - Positive Student-t copula degrees of freedom controlling tail thickness.
   * @throws Error - Throws a JavaScript exception if `df` is not finite and strictly greater than two.
   */
  studentT(df: number): CopulaSpec;
  /**
   * Random Factor Loading copula with stochastic correlation.
   * @returns Returns the resulting `CopulaSpec` value or WebAssembly handle.
   * @param loadingVol - Standard deviation used to randomize the factor loading.
   */
  randomFactorLoading(loadingVol: number): CopulaSpec;
  /**
   * Multi-factor Gaussian copula with sector structure.
   * @returns Returns the resulting `CopulaSpec` value or WebAssembly handle.
   * @param numFactors - Positive number of systematic factors in the Gaussian factor model.
   */
  multiFactor(numFactors: number): CopulaSpec;
}

/**
 * Concrete recovery model for credit portfolio pricing.
 */
export interface RecoveryModel extends WasmOwned {
  /**
   * Expected (unconditional, Jensen-corrected) recovery rate.
   */
  readonly expectedRecovery: number;
  /**
   * Loss given default (1 − recovery).
   */
  readonly lgd: number;
  /**
   * Recovery-rate volatility scale (0 for constant models).
   */
  readonly recoveryVolatility: number;
  /**
   * Whether recovery varies with the market factor.
   */
  readonly isStochastic: boolean;
  /**
   * Model name for diagnostics.
   */
  readonly modelName: string;
  /**
   * Recovery conditional on the systematic market factor.
   * @returns Returns the computed numeric result in the units described above.
   * @param marketFactor - Realized standardized market factor used to condition recovery or loss given default.
   */
  conditionalRecovery(marketFactor: number): number;
  /**
   * Conditional LGD given market factor.
   * @returns Returns the computed numeric result in the units described above.
   * @param marketFactor - Realized standardized market factor used to condition recovery or loss given default.
   */
  conditionalLgd(marketFactor: number): number;
}

/**
 * Recovery model specification for configuration and deferred construction.
 */
export interface RecoverySpec extends WasmOwned {
  /**
   * Location-parameter recovery rate of this spec.
   *
   * For a constant spec this is the constant rate. For a
   * market-correlated spec this returns the `mean` input — the target
   * recovery at factor `Z = 0` — which differs from the Jensen-corrected
   * unconditional mean `E_Z[R(Z)]` whenever the factor sensitivity is
   * non-zero. For the true unconditional mean call
   * `build().expectedRecovery`.
   */
  readonly expectedRecovery: number;
  /**
   * Build a concrete recovery model from this specification.
   * @returns Returns the resulting `RecoveryModel` value or WebAssembly handle.
   */
  build(): RecoveryModel;
}

/**
 * Recovery model specification for configuration and deferred construction.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const recovery = valuations.correlation.RecoverySpec.constant(0.4).build();
 * console.log(recovery.expectedRecovery, recovery.lgd);
 * ```
 */
export interface RecoverySpecConstructor {
  /**
   * Constant recovery rate.
   *
   * Throws if `rate` is not finite or lies outside `[0, 1]`.
   * @returns Returns the resulting `RecoverySpec` value or WebAssembly handle.
   * @param rate - Constant recovery rate expressed as a fraction from 0 through 1.
   * @throws Error - Throws a JavaScript exception if `rate` is not finite or lies outside `[0, 1]`.
   */
  constant(rate: number): RecoverySpec;
  /**
   * Market-correlated (Andersen-Sidenius) stochastic recovery.
   *
   * Throws if `mean` is not finite or lies outside `[0, 1]`, or if `vol` /
   * `correlation` are not finite.
   * @returns Returns the resulting `RecoverySpec` value or WebAssembly handle.
   * @param mean - Mean recovery rate expressed as a fraction from 0 through 1.
   * @param vol - Recovery-rate volatility scale in the correlated recovery model.
   * @param correlation - Dependence correlation from -1 through 1 under the selected copula or recovery model.
   * @throws Error - Throws a JavaScript exception if `mean` is not finite or lies outside `[0, 1]`, or if `vol` or `correlation` is non-finite. Finite volatility and correlation inputs are clamped to their supported ranges.
   */
  marketCorrelated(mean: number, vol: number, correlation: number): RecoverySpec;
  /**
   * Market-standard stochastic recovery (40% mean, 25% vol, +40% corr —
   * recovery falls in stress under the canonical low-factor-stress
   * convention).
   * @returns Returns the resulting `RecoverySpec` value or WebAssembly handle.
   */
  marketStandardStochastic(): RecoverySpec;
}

/**
 * Exported class; construct instances via `CopulaSpec.build()` (no public `new`).
 */
export interface CopulaClass {
  /**
   * Prototype exposed by this `CopulaClass` value.
   */
  readonly prototype: Copula;
}

/**
 * Exported class; construct instances via `RecoverySpec.build()` (no public `new`).
 */
export interface RecoveryModelClass {
  /**
   * Prototype exposed by this `RecoveryModelClass` value.
   */
  readonly prototype: RecoveryModel;
}

/**
 * Tranche loss statistics returned by
 * {@link CorrelationNamespace.trancheLossStatistics}.
 *
 * Fractions are expressed relative to the tranche notional unless the field
 * name says otherwise; amounts are in the same unit as the input losses.
 */
export interface TrancheLossStatisticsJson {
  /**
   * Tranche attachment point as a fraction of pool notional, in `[0, 1)`.
   */
  attachment: number;
  /**
   * Tranche detachment point as a fraction of pool notional, in `(0, 1]`.
   */
  detachment: number;
  /**
   * Tranche notional `(detachment - attachment) * poolNotional`.
   */
  tranche_notional: number;
  /**
   * Mean tranche loss as a fraction of tranche notional, in `[0, 1]`.
   */
  expected_loss_fraction: number;
  /**
   * Mean tranche loss in pool-notional units.
   */
  expected_loss_amount: number;
  /**
   * Nearest-rank tranche loss fraction at the distribution's confidence.
   */
  var_fraction: number;
  /**
   * Nearest-rank tranche loss amount at the distribution's confidence.
   */
  var_amount: number;
  /**
   * Mean tranche loss fraction from the VaR observation through the worst path.
   */
  expected_shortfall_fraction: number;
  /**
   * Mean tranche loss amount from the VaR observation through the worst path.
   */
  expected_shortfall_amount: number;
  /**
   * Share of paths whose pool loss fraction strictly exceeds `attachment`.
   */
  prob_attachment_breached: number;
  /**
   * Share of paths whose pool loss fraction reaches or exceeds `detachment`.
   */
  prob_full_writedown: number;
}

/**
 * Namespaced TypeScript entry points for correlation calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const [lower, upper] = valuations.correlation.correlationBounds(0.1, 0.2);
 * console.log(lower, upper);
 * ```
 */
export interface CorrelationNamespace {
  /**
   * Copula spec exposed by this `Correlation` value.
   */
  CopulaSpec: CopulaSpecConstructor;
  /**
   * Copula exposed by this `Correlation` value.
   */
  Copula: CopulaClass;
  /**
   * Recovery spec exposed by this `Correlation` value.
   */
  RecoverySpec: RecoverySpecConstructor;
  /**
   * Recovery model exposed by this `Correlation` value.
   */
  RecoveryModel: RecoveryModelClass;
  /**
   * Fréchet-Hoeffding correlation bounds for two Bernoulli marginals.
   *
   * Returns `[rho_min, rho_max]`.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param p1 - First marginal default probability from 0 through 1.
   * @param p2 - Second marginal default probability from 0 through 1.
   * @throws Error - Throws a JavaScript exception if either marginal probability is non-finite or outside `[0, 1]`.
   */
  correlationBounds(p1: number, p2: number): Float64Array;
  /**
   * Joint probabilities for two correlated Bernoulli variables.
   *
   * Returns `[p11, p10, p01, p00]`.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param p1 - First marginal default probability from 0 through 1.
   * @param p2 - Second marginal default probability from 0 through 1.
   * @param correlation - Dependence correlation from -1 through 1 under the selected copula or recovery model.
   * @throws Error - Throws a JavaScript exception if either marginal probability is non-finite or outside `[0, 1]`, or `correlation` is non-finite or outside `[-1, 1]`.
   */
  jointProbabilities(p1: number, p2: number, correlation: number): Float64Array;
  /**
   * Validate a flat row-major correlation matrix.
   *
   * Accepts a `Float64Array`/`number[]` of `n * n` row-major entries and
   * checks unit diagonal, off-diagonal in `[-1, 1]`, symmetry, and positive
   * semi-definiteness. Returns nothing on success; raises a descriptive error
   * (including the failing dimension or constraint) otherwise.
   * @param matrix - Square numeric matrix in the nested or row-major shape required by this callable.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @throws Error - Throws a JavaScript exception if the flat length is not `n * n`, a diagonal entry is not one, an entry is outside the correlation bounds, the matrix is not symmetric, or the matrix is not positive semidefinite.
   */
  validateCorrelationMatrix(matrix: NumericArray, n: number): void;
  /**
   * Nearest correlation matrix (Higham 2002) for a near-PSD input.
   *
   * Projects a symmetric, near-unit-diagonal, near-PSD matrix onto the set of
   * valid correlation matrices in Frobenius norm. Gross input violations
   * (asymmetry > 1e-6 or diagonal far from 1) throw rather than being silently
   * reshaped. Returns the flat row-major result as a `Float64Array`.
   */
  /**
   * Nearest correlation matrix (Higham 2002).
   *
   * Given a flat row-major `n*n` matrix that is approximately a correlation
   * matrix but fails Cholesky by a small margin, returns the nearest valid
   * correlation matrix (symmetric, unit diagonal, PSD) in Frobenius norm.
   * Gross input violations raise rather than being silently reshaped.
   * @returns Returns numeric results as a `Float64Array` in the documented order.
   * @param matrix - Square numeric matrix in the nested or row-major shape required by this callable.
   * @param n - Positive square-matrix dimension; flat arrays must contain n × n entries.
   * @param maxIter - Maximum number of Higham nearest-correlation projection iterations.
   * @param tol - Positive convergence tolerance for the nearest-correlation projection.
   * @throws Error - Throws a JavaScript exception if the flat length is not `n * n`, the input has a gross diagonal or symmetry violation, or the projection does not converge within `maxIter` iterations at `tol`.
   */
  nearestCorrelation(matrix: NumericArray, n: number, maxIter?: number, tol?: number): Float64Array;
  /**
   * Tranche loss statistics over a simulated pool loss distribution.
   *
   * `attachment` and `detachment` are fractions of pool notional in `[0, 1]` —
   * a 0-3% equity tranche is `(0.0, 0.03)`, not `(0.0, 3.0)`. Each path's pool
   * loss fraction `L = loss / poolNotional` maps through
   * `clamp(L - attachment, 0, width) / width`, and the resulting distribution
   * is aggregated at `confidence` using loss-positive nearest-rank conventions.
   * @returns Returns the tranche notional, expected loss, VaR, expected shortfall, and breach probabilities.
   * @param losses - Loss-positive path losses in one caller-defined unit, one entry per simulated path.
   * @param confidence - Loss-positive VaR and expected-shortfall confidence strictly between 0 and 1.
   * @param attachment - Lower tranche boundary as a fraction of pool notional from 0 through 1.
   * @param detachment - Upper tranche boundary as a fraction of pool notional, strictly above the attachment and at most 1.
   * @param poolNotional - Total pool notional, finite and strictly positive, in the same unit as the losses.
   * @throws Error - Throws a JavaScript exception if the loss distribution is empty or contains a non-finite or negative loss; `confidence` is outside `(0, 1)`; tranche boundaries are invalid; `poolNotional` is not finite and positive; a derived statistic is non-finite; allocation fails; or conversion to JavaScript fails.
   */
  trancheLossStatistics(
    losses: NumericArray,
    confidence: number,
    attachment: number,
    detachment: number,
    poolNotional: number
  ): TrancheLossStatisticsJson;
}

// --- monte_carlo ----------------------------------------------------------
// Convenience subset of finstack-quant-monte-carlo. Advanced Rust process,
// discretization, RNG, payoff, and Greeks types are not standalone WASM types.

/**
 * Namespaced TypeScript entry points for monte carlo calculations and types.
 * @example
 * ```typescript
 * import init, { monte_carlo } from "finstack-quant-wasm";
 * await init();
 * const estimate = monte_carlo.priceEuropeanCall(
 *   100,
 *   100,
 *   0.03,
 *   0,
 *   0.2,
 *   1,
 *   10_000,
 *   42n,
 *   64,
 *   "USD"
 * );
 * console.log(estimate.estimate);
 * ```
 */
export interface MonteCarloNamespace {
  /**
   * Price a European call option via Monte Carlo under GBM dynamics.
   *
   * Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
   * `ci_lower`, `ci_upper`, `num_paths`, `num_simulated_paths`,
   * `median`, `percentile_25`, `percentile_75`, `min`, `max`, and
   * `relative_stderr`.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; the GBM parameters, expiry, step count, path count, or computed discount factor fail validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceEuropeanCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price a European put option via Monte Carlo under GBM dynamics.
   *
   * Returns a JSON object with `mean`, `currency`, `stderr`, `std_dev`,
   * `ci_lower`, `ci_upper`, `num_paths`, `num_simulated_paths`,
   * `median`, `percentile_25`, `percentile_75`, `min`, `max`, and
   * `relative_stderr`.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; the GBM parameters, expiry, step count, path count, or computed discount factor fail validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceEuropeanPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price a European call under Heston stochastic volatility.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param kappa - Mean-reversion speed of variance in the Heston stochastic-volatility model.
   * @param theta - Long-run variance level in the Heston stochastic-volatility model.
   * @param volOfVol - Annualized volatility of variance in the Heston stochastic-volatility model.
   * @param rho - Instantaneous correlation between the asset and variance shocks.
   * @param v0 - Initial instantaneous variance in the Heston stochastic-volatility model.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; `rate` or `div_yield` is non-finite; `kappa`, `theta`, `vol_of_vol`, or `v0` is non-finite or non-positive; `rho` is outside `[-1, 1]`; the expiry, step count, path count, or computed discount factor fails validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceHestonCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    kappa: number,
    theta: number,
    volOfVol: number,
    rho: number,
    v0: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price a European put under Heston stochastic volatility.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param kappa - Mean-reversion speed of variance in the Heston stochastic-volatility model.
   * @param theta - Long-run variance level in the Heston stochastic-volatility model.
   * @param volOfVol - Annualized volatility of variance in the Heston stochastic-volatility model.
   * @param rho - Instantaneous correlation between the asset and variance shocks.
   * @param v0 - Initial instantaneous variance in the Heston stochastic-volatility model.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; `rate` or `div_yield` is non-finite; `kappa`, `theta`, `vol_of_vol`, or `v0` is non-finite or non-positive; `rho` is outside `[-1, 1]`; the expiry, step count, path count, or computed discount factor fails validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceHestonPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    kappa: number,
    theta: number,
    volOfVol: number,
    rho: number,
    v0: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price an Asian call via Monte Carlo under GBM dynamics.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; the GBM parameters, expiry, step count, path count, or computed discount factor fail validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceAsianCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price an Asian put via Monte Carlo under GBM dynamics.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @throws Error - Throws a JavaScript exception if `currency` is unknown; embedded defaults cannot be loaded when `num_steps` is omitted; the GBM parameters, expiry, step count, path count, or computed discount factor fail validation; a simulated discounted payoff is non-finite; or the result cannot be serialized.
   */
  priceAsianPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  /**
   * Price an American put via LSMC under GBM dynamics.
   *
   * Optional knobs:
   * - `use_parallel` (default `false`): run path generation on the rayon pool.
   * - `basis` (default `"laguerre"`): regression basis — `"laguerre"`,
   *   `"polynomial"`, or `"normalized_polynomial"`.
   * - `basis_degree` (default `3`): polynomial/Laguerre degree. Must be
   *   positive; `"laguerre"` additionally requires degree in `[1, 4]`.
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @param useParallel - Whether simulation paths are evaluated in parallel when supported.
   * @param basis - Regression basis family used by the American-option exercise estimator.
   * @param basisDegree - Maximum polynomial degree used by the American-option exercise basis.
   * @throws Error - Throws a JavaScript exception if the embedded defaults cannot be loaded; `currency` is unknown; `strike <= 0`; the GBM parameters, path count, step count, expiry, basis name, or basis degree fail validation; path generation fails; or the result cannot be serialized.
   */
  priceAmericanPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  /**
   * Price an American call via LSMC under GBM dynamics.
   *
   * Optional knobs match [`price_american_put`].
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @param useParallel - Whether simulation paths are evaluated in parallel when supported.
   * @param basis - Regression basis family used by the American-option exercise estimator.
   * @param basisDegree - Maximum polynomial degree used by the American-option exercise basis.
   * @throws Error - Throws a JavaScript exception if the embedded defaults cannot be loaded; `currency` is unknown; `strike <= 0`; the GBM parameters, path count, step count, expiry, basis name, or basis degree fail validation; path generation fails; or the result cannot be serialized.
   */
  priceAmericanCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  /**
   * Two-pass unbiased American put price (training fit + out-of-sample pricing).
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param pricingSeed - Independent deterministic seed used for unbiased-pricing sampling.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @param useParallel - Whether simulation paths are evaluated in parallel when supported.
   * @param basis - Regression basis family used by the American-option exercise estimator.
   * @param basisDegree - Maximum polynomial degree used by the American-option exercise basis.
   * @throws Error - Throws a JavaScript exception if the embedded defaults cannot be loaded; `currency` is unknown; `strike <= 0`; the GBM parameters, path count, step count, expiry, basis name, or basis degree fail validation; `pricing_seed == seed`; either path-generation pass or the regression fit fails; or the result cannot be serialized.
   */
  priceAmericanPutUnbiased(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    pricingSeed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  /**
   * Two-pass unbiased American call price (training fit + out-of-sample pricing).
   * @returns Returns the resulting `MonteCarloEstimateJson` value or WebAssembly handle.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   * @param numPaths - Number of simulated stochastic paths; larger values improve sampling precision.
   * @param seed - Deterministic random-number seed used to reproduce simulation output.
   * @param pricingSeed - Independent deterministic seed used for unbiased-pricing sampling.
   * @param numSteps - Number of time steps per simulated path.
   * @param currency - ISO-4217 currency code for the monetary amount or market convention.
   * @param useParallel - Whether simulation paths are evaluated in parallel when supported.
   * @param basis - Regression basis family used by the American-option exercise estimator.
   * @param basisDegree - Maximum polynomial degree used by the American-option exercise basis.
   * @throws Error - Throws a JavaScript exception if the embedded defaults cannot be loaded; `currency` is unknown; `strike <= 0`; the GBM parameters, path count, step count, expiry, basis name, or basis degree fail validation; `pricing_seed == seed`; either path-generation pass or the regression fit fails; or the result cannot be serialized.
   */
  priceAmericanCallUnbiased(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    pricingSeed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  /**
   * Black-Scholes call price.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   */
  blackScholesCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number
  ): number;
  /**
   * Black-Scholes put price.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param expiry - Time to option expiry in years on the model's annual time basis.
   */
  blackScholesPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number
  ): number;
}

/**
 * Namespaced TypeScript entry point for monte carlo APIs.
 */
export declare const monte_carlo: MonteCarloNamespace;

// --- margin ----------------------------------------------------------------

/**
 * Namespaced TypeScript entry points for margin calculations and types.
 * @example
 * ```typescript
 * import init, { margin } from "finstack-quant-wasm";
 * await init();
 * const csa = margin.csaUsdRegulatory();
 * const vm = margin.calculateVm(csa, 1_000_000, 0, "USD", 2026, 1, 2);
 * console.log(vm.call_amount);
 * ```
 */
export interface MarginNamespace {
  /**
   * Create a standard USD regulatory CSA specification as JSON.
   *
   * Returns the canonical ISDA-compliant CSA for USD OTC derivatives.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Rejects if the embedded margin registry cannot be loaded or the resulting CSA cannot be serialized to JSON.
   */
  csaUsdRegulatory(): string;
  /**
   * Create a standard EUR regulatory CSA specification as JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Rejects if the embedded margin registry cannot be loaded or the resulting CSA cannot be serialized to JSON.
   */
  csaEurRegulatory(): string;
  /**
   * Validate a CSA specification JSON string.
   *
   * Deserializes and re-serializes the input to verify it conforms
   * to the `CsaSpec` schema. Returns the canonical JSON on success.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - CSA specification JSON to validate and normalize into canonical form.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or failure to serialize the decoded CSA specification.
   */
  validateCsaJson(json: string): string;
  /**
   * Calculate variation margin given exposure, posted collateral, and CSA JSON.
   *
   * Returns a JSON object with delivery_amount, return_amount, net_exposure,
   * and requires_call fields.
   *
   * @returns Returns the resulting `VariationMarginJson` value or WebAssembly handle.
   * @param csaJson - CSA specification as JSON string
   * @param exposure - Current mark-to-market exposure amount
   * @param postedCollateral - Currently posted collateral amount
   * @param currency - ISO currency code (e.g. "USD")
   * @param year - Calculation year
   * @param month - Calculation month (1-12)
   * @param day - Calendar day number within the selected month of the VM calculation date.
   * @param csaJson - CSA specification JSON governing thresholds, minimum transfer, and timing.
   * @param exposure - Current mark-to-market exposure in the supplied currency units.
   * @param postedCollateral - Collateral already posted in the supplied currency units.
   * @param currency - ISO-4217 currency code shared by exposure and collateral amounts.
   * @param year - Calendar year of the VM calculation date.
   * @param month - Calendar month from 1 through 12 of the VM calculation date.
   * @param day - Calendar day number within the selected month of the VM calculation date.
   * @throws Error - Rejects malformed or schema-incompatible `csa_json`, an unknown `currency`, non-finite exposure or collateral amounts, an invalid calendar date, a currency mismatch with the CSA, invalid VM parameters, calendar lookup or settlement-date adjustment failures, or failure to serialize the result.
   */
  calculateVm(
    csaJson: string,
    exposure: number,
    postedCollateral: number,
    currency: string,
    year: number,
    month: number,
    day: number
  ): VariationMarginJson;
  /**
   * Compute bilateral XVA: CVA, DVA, FVA, MVA, and the all-in adjustment.
   *
   * All legs are weighted by joint (first-to-default) survival. MVA is computed
   * only when `fundingJson` carries an `im_profile`; that posted IM also reduces
   * ENE for bilateral DVA.
   *
   * The returned object reports the required all-in amount as
   * `total_xva = CVA - DVA + FVA + MVA`. Optional funding legs are absent from
   * the payload when they were not computed.
   *
   * @example
   * ```javascript
   * import init, { core, margin } from "finstack-quant-wasm";
   * await init();
   * const df = new core.DiscountCurve("USD-OIS", "2025-01-01", [0.0, 1.0, 5.0, 1.0], "log_linear");
   * const hz = new core.HazardCurve("CPTY", "2025-01-01", [0.0, 0.02, 30.0, 0.02], 0.4);
   * const result = margin.computeBilateralXva(
   *   JSON.stringify({ times: [1, 2], mtm_values: [1e6, 1e6], epe: [1e6, 1e6], ene: [0, 0] }),
   *   hz, hz, df, 0.4, 0.4,
   *   JSON.stringify({ funding_spread_bp: 50.0 }),
   * );
   * result.total_xva; // CVA - DVA + FVA + MVA
   * ```
   * @param exposureProfileJson - `ExposureProfile` JSON with `times`,
   * @param counterpartyHazardCurve - Hazard curve for the counterparty's credit.
   * @param ownHazardCurve - Hazard curve for the institution's own credit.
   * @param discountCurve - Risk-free discount curve for present-valuing.
   * @param counterpartyRecoveryRate - Recovery on counterparty default, in `[0, 1]`.
   * @param ownRecoveryRate - Recovery on own default, in `[0, 1]`.
   * @param fundingJson - Optional strict `FundingConfig` JSON driving FVA and,
   * @returns The `XvaResult` as a plain object.
   * @throws Error - If JSON is malformed or has unknown funding fields, a recovery rate
   */
  computeBilateralXva(
    exposureProfileJson: string,
    counterpartyHazardCurve: HazardCurve,
    ownHazardCurve: HazardCurve,
    discountCurve: DiscountCurve,
    counterpartyRecoveryRate: number,
    ownRecoveryRate: number,
    fundingJson?: string | null
  ): XvaResultJson;
}

/**
 * Namespaced TypeScript entry point for margin APIs.
 */
export declare const margin: MarginNamespace;

// --- cashflows -------------------------------------------------------------

/**
 * JSON bridge to the Rust `finstack-quant-cashflows` crate.
 *
 * All methods accept and return JSON strings that mirror the canonical Rust
 * serde model. Cashflow JSON types are exported from `./types`.
 * @example
 * ```typescript
 * import init, { cashflows } from "finstack-quant-wasm";
 * await init();
 * const spec = JSON.stringify({
 *   notional: { initial: { amount: "1000000", currency: "USD" }, amort: "none" },
 *   issue: "2026-01-02",
 *   maturity: "2027-01-02",
 *   coupon_program: [{
 *     kind: "fixed",
 *     spec: {
 *       coupon_type: "cash",
 *       rate: "0.05",
 *       frequency: { count: 12, unit: "months" },
 *       day_count: "30_360",
 *       business_day_convention: "following",
 *       calendar_id: "weekends_only",
 *       stub: "none",
 *       end_of_month: false,
 *       payment_lag_days: 0
 *     }
 *   }]
 * });
 * const schedule = cashflows.buildCashflowScheduleJson(spec);
 * console.log(JSON.parse(cashflows.datedFlowsJson(schedule)).length);
 * ```
 */
export interface CashflowsNamespace {
  /**
   * Build a cashflow schedule from a `CashflowScheduleBuildSpec` JSON string.
   *
   * @param specJson - JSON-encoded `CashflowScheduleBuildSpec`.
   * @param marketJson - Optional JSON-encoded market context for floating-rate lookups.
   * @returns JSON-encoded `CashFlowSchedule`.
   * @throws If the spec or market JSON is malformed, or schedule construction fails.
   */
  buildCashflowScheduleJson(specJson: string, marketJson?: string | null): string;

  /**
   * Validate a cashflow schedule JSON string and return it canonicalized.
   *
   * @param scheduleJson - JSON-encoded `CashFlowSchedule`.
   * @returns Canonicalized JSON-encoded `CashFlowSchedule`.
   * @throws If the schedule JSON is malformed or fails validation.
   */
  validateCashflowScheduleJson(scheduleJson: string): string;

  /**
   * Extract dated flows from a cashflow schedule JSON string.
   *
   * @param scheduleJson - JSON-encoded `CashFlowSchedule`.
   * @returns JSON array of settlement cash entries. PIK and
   * @throws If the schedule JSON is malformed.
   */
  datedFlowsJson(scheduleJson: string): string;

  /**
   * Compute accrued interest from a cashflow schedule JSON string as of a given date.
   *
   * @param scheduleJson - JSON-encoded `CashFlowSchedule`.
   * @param asOf - ISO-8601 date (YYYY-MM-DD) for the accrual snapshot.
   * @param configJson - Optional JSON-encoded `AccrualConfig` overriding defaults.
   * @returns Accrued interest in the schedule's settlement currency as a JS
   * @throws If any JSON input is malformed or the accrual computation fails.
   */
  accruedInterestJson(scheduleJson: string, asOf: string, configJson?: string | null): number;
}

/**
 * Namespaced TypeScript entry point for cashflows APIs.
 */
export declare const cashflows: CashflowsNamespace;

// --- covenants -------------------------------------------------------------

/**
 * JSON bridge to the Rust `finstack-quant-covenants` crate.
 * @example
 * ```typescript
 * import init, { covenants } from "finstack-quant-wasm";
 * await init();
 * const engine = JSON.parse(covenants.lboStandard(6, 2, 1.5, 50_000_000));
 * console.log(engine);
 * ```
 */
export interface CovenantsNamespace {
  /**
   * Validate and canonicalize a covenant spec JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - JSON-serialized covenant specification to validate.
   * @throws Error - Throws a JavaScript exception if `specJson` is malformed, does not match the covenant-spec schema, violates covenant threshold or frequency invariants, or cannot be serialized to canonical JSON.
   */
  validateCovenantSpec(specJson: string): string;
  /**
   * Validate and canonicalize a covenant report JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param reportJson - JSON-serialized covenant evaluation report to validate.
   * @throws Error - Throws a JavaScript exception if `reportJson` is malformed, does not match the covenant-report schema, or cannot be serialized to canonical JSON.
   */
  validateCovenantReport(reportJson: string): string;
  /**
   * Validate and canonicalize a covenant engine JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param engineJson - JSON-serialized covenant engine and its covenant definitions.
   * @throws Error - Throws a JavaScript exception if `engineJson` is malformed, does not match the covenant-engine schema, contains an invalid covenant package, violates engine invariants, or cannot be serialized to canonical JSON.
   */
  validateCovenantEngine(engineJson: string): string;
  /**
   * Evaluate a covenant engine JSON string against a JSON metric map.
   * @returns Returns the requested string representation or JSON payload.
   * @param engineJson - JSON-serialized covenant engine and its covenant definitions.
   * @param metricsJson - JSON object of financial metrics referenced by the covenant engine.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed or has the wrong schema, a metric is non-numeric, `asOf` is not a valid ISO date, the engine or required metrics fail validation, or the report cannot be serialized.
   */
  evaluateEngine(engineJson: string, metricsJson: string, asOf: string): string;
  /**
   * Standard leveraged-buyout covenant package as JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param initialLeverage - Maximum leverage ratio permitted at the initial test date.
   * @param interestCoverage - Minimum EBITDA-to-cash-interest coverage ratio.
   * @param fixedChargeCoverage - Minimum EBITDA-to-fixed-charges coverage ratio.
   * @param maxCapex - Maximum capital expenditure amount or ratio in the covenant convention.
   * @throws Error - Throws a JavaScript exception if the generated covenant package cannot be serialized to JSON.
   */
  lboStandard(
    initialLeverage: number,
    interestCoverage: number,
    fixedChargeCoverage: number,
    maxCapex: number
  ): string;
  /**
   * Covenant-lite package as JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param maxLeverage - Maximum total debt-to-EBITDA leverage ratio.
   * @param maxSeniorLeverage - Maximum senior-debt-to-EBITDA leverage ratio.
   * @throws Error - Throws a JavaScript exception if the generated covenant package cannot be serialized to JSON.
   */
  covLite(maxLeverage: number, maxSeniorLeverage: number): string;
  /**
   * Real-estate covenant package as JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param minDscr - Minimum debt-service coverage ratio.
   * @param minDebtYield - Minimum net-operating-income debt yield expressed as a decimal.
   * @param maxLtv - Maximum loan-to-value ratio expressed as a decimal.
   * @throws Error - Throws a JavaScript exception if the generated covenant package cannot be serialized to JSON.
   */
  realEstate(minDscr: number, minDebtYield: number, maxLtv: number): string;
  /**
   * Project-finance covenant package as JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param minDscr - Minimum debt-service coverage ratio.
   * @param distributionLockupDscr - DSCR threshold below which borrower distributions are locked up.
   * @param minLiquidity - Minimum required liquidity reserve in the model's monetary units.
   * @param maxNetLeverage - Maximum net-debt-to-EBITDA leverage ratio.
   * @throws Error - Throws a JavaScript exception if the generated covenant package cannot be serialized to JSON.
   */
  projectFinance(
    minDscr: number,
    distributionLockupDscr: number,
    minLiquidity: number,
    maxNetLeverage: number
  ): string;
}

/**
 * Namespaced TypeScript entry point for covenants APIs.
 */
export declare const covenants: CovenantsNamespace;

// --- valuations ------------------------------------------------------------

export declare class Market {
  constructor(json: string);
  toJson(): string;
}

/**
 * TypeScript view of the typed `Bond` WebAssembly instrument.
 *
 * Thin wrapper over the canonical Rust `Bond`. Serialize with `toJson()` and
 * pass the result to `valuations.instruments.priceInstrument` (or the other
 * generic pricing entry points) to price it.
 */
export interface Bond extends WasmOwned {
  /**
   * Instrument identifier.
   * @returns Returns the requested string representation or JSON payload.
   */
  readonly id: string;
  /**
   * Serialize to a canonical `finstack_quant.instrument/1` envelope.
   *
   * Pass the result to `valuations.instruments.priceInstrument` (or the
   * other generic pricing entry points) to price this bond.
   * @returns Canonical instrument envelope accepted by `priceInstrument` and `Bond.fromJson`.
   * @throws If serialization fails.
   */
  toJson(): string;
}

/**
 * Constructor surface for the typed `Bond` WebAssembly instrument.
 * @example
 * ```typescript
 * import init, { core, valuations } from "finstack-quant-wasm";
 * await init();
 * const usd = new core.Currency("USD");
 * const bond = valuations.instruments.Bond.fixed(
 *   "BOND-1",
 *   new core.Money(1_000_000, usd),
 *   new core.Rate(0.05),
 *   "2024-01-01",
 *   "2034-01-01",
 *   "USD-OIS"
 * );
 * const result = valuations.instruments.priceInstrument(bond.toJson(), marketJson, "2024-06-30", "default");
 * ```
 */
export interface BondConstructor {
  /**
   * Create a standard fixed-rate bond (semi-annual, 30/360, T+2). Mirrors Rust `Bond::fixed`.
   * @param id - Unique instrument identifier.
   * @param notional - Principal amount of the bond.
   * @param couponRate - Annual coupon rate.
   * @param issue - Issue date as an ISO-8601 string (`"YYYY-MM-DD"`).
   * @param maturity - Maturity date as an ISO-8601 string (`"YYYY-MM-DD"`).
   * @param discountCurveId - Discount curve identifier used for pricing.
   * @returns The validated fixed-rate bond.
   * @throws If validation fails (e.g. maturity not after issue).
   */
  fixed(
    id: string,
    notional: Money,
    couponRate: Rate,
    issue: string,
    maturity: string,
    discountCurveId: string
  ): Bond;
  /**
   * Create a floating-rate bond (FRN) linked to a forward index. Mirrors Rust `Bond::floating`.
   * @param id - Unique instrument identifier.
   * @param notional - Principal amount of the bond.
   * @param indexId - Forward curve identifier (e.g. `"USD-SOFR-3M"`).
   * @param marginBp - Spread over the index in whole basis points
   * @param issue - Issue date as an ISO-8601 string (`"YYYY-MM-DD"`).
   * @param maturity - Maturity date as an ISO-8601 string (`"YYYY-MM-DD"`).
   * @param frequency - Payment frequency (e.g. `Tenor.quarterly()`).
   * @param dayCount - Day count convention (e.g. `DayCount.act360()`).
   * @param discountCurveId - Discount curve identifier used for pricing.
   * @returns The validated floating-rate note.
   * @throws If validation fails.
   */
  floating(
    id: string,
    notional: Money,
    indexId: string,
    marginBp: Bps,
    issue: string,
    maturity: string,
    frequency: Tenor,
    dayCount: DayCount,
    discountCurveId: string
  ): Bond;
  /**
   * Deserialize a bond from its canonical v1 instrument envelope.
   *
   * Bare payloads are rejected; the loader's validation runs on the result.
   * @param json - A `finstack_quant.instrument/1` envelope containing type `"bond"`.
   * @returns The validated bond.
   * @throws If the JSON is malformed, has a different instrument type, or fails validation.
   */
  fromJson(json: string): Bond;
}

/**
 * TypeScript view of the typed `TermLoan` WebAssembly instrument.
 *
 * Thin wrapper over the canonical Rust `TermLoan`. Serialize with `toJson()`
 * and pass the result to `valuations.instruments.priceInstrument` (or the
 * other generic pricing entry points) to price it.
 */
export interface TermLoan extends WasmOwned {
  /**
   * Instrument identifier.
   * @returns Returns the requested string representation or JSON payload.
   */
  readonly id: string;
  /**
   * Serialize to a canonical `finstack_quant.instrument/1` envelope.
   *
   * Pass the result to `valuations.instruments.priceInstrument` (or the
   * other generic pricing entry points) to price this loan.
   * @returns Canonical instrument envelope accepted by `priceInstrument` and `TermLoan.fromJson`.
   * @throws If serialization fails.
   */
  toJson(): string;
}

/**
 * Constructor surface for the typed `TermLoan` WebAssembly instrument.
 *
 * Rust has no `fixed`/`floating` convenience constructors for term loans;
 * construct via `fromJson` with a canonical v1 instrument envelope or start
 * from `example()`.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const loan = valuations.instruments.TermLoan.example();
 * const result = valuations.instruments.priceInstrument(loan.toJson(), marketJson, "2024-06-30", "default");
 * ```
 */
export interface TermLoanConstructor {
  /**
   * Deserialize a term loan from its canonical v1 instrument envelope.
   *
   * Bare payloads are rejected; the loader's validation runs on the result.
   * @param json - A `finstack_quant.instrument/1` envelope containing type `"term_loan"`.
   * @returns The validated term loan.
   * @throws If the JSON is malformed, has a different instrument type, or fails validation.
   */
  fromJson(json: string): TermLoan;
  /**
   * Canonical example term loan (mirrors Rust `TermLoan::example`).
   *
   * Returns a 5-year USD fixed-rate loan (6%, quarterly, Act/360, 2.5%
   * per-period amortization) useful as a starting point and in tests.
   * @returns The example loan.
   * @throws If construction fails (should not occur).
   */
  example(): TermLoan;
}

/**
 * Namespaced TypeScript entry points for valuation instruments calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const models = valuations.instruments.listModels();
 * console.log(models.includes("discounting"));
 * ```
 */
export interface ValuationInstrumentsNamespace {
  /**
   * Typed `Bond` instrument class (see `BondConstructor`).
   */
  Bond: BondConstructor;
  /**
   * Typed `TermLoan` instrument class (see `TermLoanConstructor`).
   */
  TermLoan: TermLoanConstructor;
  /**
   * Construct a canonical bond instrument envelope from a cashflow schedule.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentId - Stable instrument identifier used for pricing and metric keys.
   * @param scheduleJson - Canonical cashflow-schedule JSON used to construct the fixed-income instrument.
   * @param discountCurveId - Market-context discount-curve identifier for the instrument currency.
   * @param quotedClean - Optional observed clean bond price in the schedule's documented price quotation convention.
   * @throws Error - Throws a JavaScript exception if `scheduleJson` is malformed or violates cash-flow invariants, bond construction fails, or the canonical bond envelope cannot be serialized.
   */
  bondFromCashflowsJson(
    instrumentId: string,
    scheduleJson: string,
    discountCurveId: string,
    quotedClean?: number | null
  ): string;
  /**
   * Validate a canonical v1 instrument envelope.
   *
   * Bare instrument payloads are rejected. Returns canonical re-serialized JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Required `finstack_quant.instrument/1` envelope.
   * @throws Error - Throws a JavaScript exception if `json` is malformed, is not a canonical v1 instrument envelope, fails instrument validation, or cannot be canonically serialized.
   */
  validateInstrumentJson(json: string): string;
  /**
   * Price an instrument from its canonical envelope and return a ValuationResult JSON.
   *
   * Pass `model = "default"` to use the instrument-native default model.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Required `finstack_quant.instrument/1` envelope.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Optional pricing-model identifier; omit for the instrument-native model.
   * @throws Error - Throws a JavaScript exception if the instrument or market JSON, `asOf`, or `model` is invalid; required market data is missing; the selected pricer fails; or the valuation cannot be serialized.
   */
  priceInstrument(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model?: string | null
  ): string;
  /**
   * Price an instrument with explicit metric requests.
   *
   * Omit `model` (or pass `"default"`) for the instrument-native default
   * model, and omit `metrics` for none — matching the Python binding's
   * `model="default"`, `metrics=[]` defaults.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Required `finstack_quant.instrument/1` envelope.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Optional pricing-model identifier; omit for the instrument-native model.
   * @param metrics - Optional array of canonical metric identifiers to calculate with the instrument price.
   * @param pricingOptions - Optional JSON pricing overrides accepted by the canonical instrument validator.
   * @param marketHistory - Optional serialized historical market snapshots required by historical pricing models.
   * @throws Error - Throws a JavaScript exception if an instrument, market, pricing-option, or market-history payload is invalid; `metrics` is not a string array; `asOf`, `model`, or a metric identifier is invalid; required market data is missing; pricing or a metric calculation fails; or the valuation cannot be serialized.
   */
  priceInstrumentWithMetrics(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model?: string | null,
    metrics?: string[] | null,
    pricingOptions?: string | null,
    marketHistory?: string | null
  ): string;
  /**
   * Price an instrument using a pre-parsed [`Market`].
   *
   * Avoids the per-call market-parse overhead of `priceInstrument`.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param market - Market context or JSON payload supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Pricing-model identifier; use `"default"` for the instrument-native model when supported.
   * @throws Error - Throws a JavaScript exception if `instrumentJson`, `asOf`, or `model` is invalid; required market data is missing; the selected pricer fails; or the valuation cannot be serialized.
   */
  priceInstrumentWithMarket(
    instrumentJson: string,
    market: Market,
    asOf: string,
    model: string
  ): string;
  /**
   * Price an instrument with explicit metric requests using a pre-parsed [`Market`].
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param market - Market context or JSON payload supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Pricing-model identifier; use `"default"` for the instrument-native model when supported.
   * @param metrics - Array of canonical metric identifiers to calculate with the instrument price.
   * @param pricingOptions - Optional JSON pricing overrides accepted by the canonical instrument validator.
   * @param marketHistory - Optional serialized historical market snapshots required by historical pricing models.
   * @throws Error - Throws a JavaScript exception if an instrument, pricing-option, or market- history payload is invalid; `metrics` is not a string array; `asOf`, `model`, or a metric identifier is invalid; required market data is missing; pricing or a metric calculation fails; or the valuation cannot be serialized.
   */
  priceInstrumentWithMetricsAndMarket(
    instrumentJson: string,
    market: Market,
    asOf: string,
    model: string,
    metrics: string[],
    pricingOptions?: string | null,
    marketHistory?: string | null
  ): string;
  /**
   * Per-flow cashflow envelope (DF / survival / PV) for a discountable instrument.
   *
   * `model` must be `"discounting"` or `"hazard_rate"`. Unsupported models or
   * incompatible instrument types throw. For supported pairs, the envelope's
   * `total_pv` matches the instrument's `base_value` within rounding.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Required `finstack_quant.instrument/1` envelope.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Pricing-model identifier; use `"default"` for the instrument-native model when supported.
   * @throws Error - Throws a JavaScript exception if the instrument or market JSON or `asOf` is invalid, `model` is unsupported or incompatible with the instrument, required curves are missing, the schedule mixes currencies, canonical pricing fails, or the cash-flow envelope cannot be serialized.
   */
  instrumentCashflowsJson(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string
  ): string;
  /**
   * Per-flow cashflow envelope using a pre-parsed [`Market`].
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param market - Market context or JSON payload supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param model - Pricing-model identifier; use `"default"` for the instrument-native model when supported.
   * @throws Error - Throws a JavaScript exception if `instrumentJson` or `asOf` is invalid, `model` is unsupported or incompatible with the instrument, required curves are missing, the schedule mixes currencies, canonical pricing fails, or the cash-flow envelope cannot be serialized.
   */
  instrumentCashflowsWithMarket(
    instrumentJson: string,
    market: Market,
    asOf: string,
    model: string
  ): string;
  /**
   * List every pricing model key registered in the standard pricer registry.
   *
   * The list is registry-derived rather than enum-derived, so it reflects real
   * dispatch coverage: a model with no registered pricer is omitted. Returns a
   * sorted array of canonical keys (`"discounting"`, `"black76"`, …) accepted
   * by the `model` argument of `priceInstrument`.
   * @returns Returns the resulting `string[]` collection in ascending model-key order.
   * @throws Error - Throws a JavaScript exception if the model key list cannot be converted to a JavaScript value.
   */
  listModels(): string[];
  /**
   * List the standard registry's pricing models grouped by instrument type.
   *
   * Returns a JSON object `{ instrument_type: [model_key, ...], ... }`. Only
   * instrument types with at least one registered pricer appear, and each
   * entry lists only the models that can actually price that instrument.
   * @returns Returns the resulting `Record<string, string[]>` value keyed by instrument type.
   * @throws Error - Throws a JavaScript exception if the grouped model registry cannot be converted to a JavaScript value.
   */
  listModelsGrouped(): Record<string, string[]>;
  /**
   * List all metric IDs in the standard metric registry.
   * @returns Returns the resulting `string[]` collection in the documented order.
   * @throws Error - Throws a JavaScript exception if the metric identifier list cannot be converted to a JavaScript value.
   */
  listStandardMetrics(): string[];
  /**
   * List all standard metrics organized by group.
   *
   * Returns a JSON object `{ group_name: [metric_id, ...], ... }` where
   * each key is a human-readable group name (e.g. "Pricing", "Greeks",
   * "Sensitivity") and the value is a sorted array of metric ID strings.
   * @returns Returns the resulting `Record<string, string[]>` value or WebAssembly handle.
   * @throws Error - Throws a JavaScript exception if the grouped metric registry cannot be converted to a JavaScript value.
   */
  listStandardMetricsGrouped(): Record<string, string[]>;
  /**
   * Z-spread-equivalent discount margin for a floating-rate tranche, returned in
   * decimal units (`0.015` = 150 bp).
   *
   * Contractual cashflows are projected without changing coupon projection,
   * then a constant additive spread is applied to the discount curve. The result
   * is zero at model PV, negative for a richer (higher) `targetPv`, and positive
   * for a cheaper (lower) `targetPv`; it is not the contractual quoted margin.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param trancheId - Identifier of the floating-rate tranche whose contractual cashflows are spread-discounted.
   * @param marketJson - Canonical market-context JSON supplying the discount curve and any forward curves or historical fixings required for cashflow projection.
   * @param asOf - ISO-8601 valuation date used for projection and discounting.
   * @param targetPv - Target present value in the tranche's currency; values above model PV produce a negative result and values below model PV produce a positive result.
   * @returns The z-spread-equivalent discount margin in decimal units.
   * @throws Error - Thrown if JSON or the date is malformed, the deal is invalid, the tranche is missing or fixed-rate, target_pv is non-finite, required market data is unavailable, or the spread solve fails or exceeds ±5000 bp.
   */
  structuredCreditTrancheDiscountMargin(
    instrumentJson: string,
    trancheId: string,
    marketJson: string,
    asOf: string,
    targetPv: number
  ): number;
  /**
   * Break-even constant default rate (CDR, decimal) for a tranche — the highest
   * CDR at which the tranche takes no principal writedown.
   * @returns Returns the computed numeric result in the units described above.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param trancheId - Stable tranche identifier used to select the required domain object.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Throws a JavaScript exception if the instrument or market JSON is malformed; the instrument fails pricing validation or is not a structured-credit deal; `as_of` is invalid; the tranche or required market data is missing; or the break-even calculation fails.
   */
  structuredCreditTrancheBreakevenCdr(
    instrumentJson: string,
    trancheId: string,
    marketJson: string,
    asOf: string
  ): number;
  /**
   * Option-adjusted spread for a tranche; returns a JSON `OasResult`.
   *
   * `marketPricePct` is the quoted price as a percentage of original balance.
   * `config`, when present, is a JSON `OasConfig`; the default is used otherwise.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param trancheId - Stable tranche identifier used to select the required domain object.
   * @param marketPricePct - Tranche market price as a percentage of original balance.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param config - Optional OasConfig JSON; omit to use the default OAS solver configuration.
   * @throws Error - Throws a JavaScript exception if the instrument, market, or optional configuration JSON is malformed; the instrument fails pricing validation; `as_of` is invalid; the tranche or discount curve is missing; the OAS solve fails or produces a non-finite result; or the result cannot be serialized.
   */
  structuredCreditTrancheOas(
    instrumentJson: string,
    trancheId: string,
    marketPricePct: number,
    marketJson: string,
    asOf: string,
    config?: string | null
  ): string;
  /**
   * Scenario (CPR x CDR x severity) table for a tranche; returns a JSON
   * `ScenarioTable`. `grid` is a JSON `ScenarioGrid` (`cprs`, `cdrs`,
   * `severities`).
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param trancheId - Stable tranche identifier used to select the required domain object.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param grid - ScenarioGrid JSON containing the CPR, CDR, and severity axes for the table.
   * @throws Error - Throws a JavaScript exception if the instrument, market, or scenario-grid JSON is malformed; the instrument fails pricing validation; `as_of` is invalid; the tranche or required market data is missing; a scenario fails or produces a non-finite result; or the table cannot be serialized.
   */
  structuredCreditTrancheScenarioTable(
    instrumentJson: string,
    trancheId: string,
    marketJson: string,
    asOf: string,
    grid: string
  ): string;
  /**
   * Per-tranche risk/spread metrics (PV, price, WAL, z-spread, CS01, spread/
   * modified duration, convexity) computed from one tranche's own cashflows.
   *
   * `marketPricePct`, when provided, is the quoted price (% of original balance)
   * the z-spread and CS01 are solved against; otherwise the tranche's own model
   * price is used (zero z-spread). Returns a JSON-serialized `TrancheMetrics`.
   * @returns Returns the requested string representation or JSON payload.
   * @param instrumentJson - Canonical JSON payload representing the instrument consumed by this API.
   * @param trancheId - Stable tranche identifier used to select the required domain object.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param marketPricePct - Optional tranche market price as a percentage of original balance; omit for model price.
   * @throws Error - Throws a JavaScript exception if the instrument or market JSON is malformed; the instrument fails pricing validation; `as_of` is invalid; the tranche or discount curve is missing; a metric fails or is non-finite; or the result cannot be serialized.
   */
  structuredCreditTrancheMetrics(
    instrumentJson: string,
    trancheId: string,
    marketJson: string,
    asOf: string,
    marketPricePct?: number | null
  ): string;
}

/**
 * TypeScript type that constrains the accepted fx instrument spec values.
 */
export type FxInstrumentSpec = Record<string, unknown> | string;

/**
 * TypeScript view of the `FxInstrument` WebAssembly value.
 */
export interface FxInstrument extends WasmOwned {
  /**
   * Serialize this `FxInstrument` value to canonical JSON.
   * @returns Returns the requested string representation or JSON payload.
   */
  toJson(): string;
  /**
   * Perform price for this `FxInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the requested string representation or JSON payload.
   */
  price(marketJson: string, asOf: string, model?: string | null): string;
  /**
   * WASM order keeps optional arguments trailing: metrics precedes model.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param metrics - Metric keys or values included in the requested calculation.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @param pricingOptions - Pricing options that select calculation behavior and output detail.
   * @param marketHistory - Chronological market snapshots used to project or backtest the result.
   * @returns Returns the requested string representation or JSON payload.
   */
  priceWithMetrics(
    marketJson: string,
    asOf: string,
    metrics: string[],
    model?: string | null,
    pricingOptions?: string | null,
    marketHistory?: string | null
  ): string;
}

/**
 * TypeScript view of the `FxOptionInstrument` WebAssembly value.
 */
export interface FxOptionInstrument extends FxInstrument {
  /**
   * Perform delta for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  delta(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform gamma for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  gamma(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform vega for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  vega(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform theta for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  theta(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform rho for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  rho(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform foreign rho for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  foreignRho(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform vanna for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  vanna(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform volga for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  volga(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform greeks for this `FxOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the resulting `Record<string, number>` value or WebAssembly handle.
   */
  greeks(marketJson: string, asOf: string, model?: string | null): Record<string, number>;
}

/**
 * TypeScript view of the `FxDigitalOptionInstrument` WebAssembly value.
 */
export interface FxDigitalOptionInstrument extends FxInstrument {
  /**
   * Perform delta for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  delta(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform gamma for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  gamma(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform vega for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  vega(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform theta for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  theta(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform rho for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  rho(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform greeks for this `FxDigitalOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the resulting `Record<string, number>` value or WebAssembly handle.
   */
  greeks(marketJson: string, asOf: string, model?: string | null): Record<string, number>;
}

/**
 * TypeScript view of the `FxTouchOptionInstrument` WebAssembly value.
 */
export interface FxTouchOptionInstrument extends FxInstrument {
  /**
   * Perform delta for this `FxTouchOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  delta(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform gamma for this `FxTouchOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  gamma(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform vega for this `FxTouchOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  vega(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform rho for this `FxTouchOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  rho(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform greeks for this `FxTouchOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the resulting `Record<string, number>` value or WebAssembly handle.
   */
  greeks(marketJson: string, asOf: string, model?: string | null): Record<string, number>;
}

/**
 * TypeScript view of the `FxBarrierOptionInstrument` WebAssembly value.
 */
export interface FxBarrierOptionInstrument extends FxTouchOptionInstrument {
  /**
   * Perform vanna for this `FxBarrierOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  vanna(marketJson: string, asOf: string, model?: string | null): number;
  /**
   * Perform volga for this `FxBarrierOptionInstrument` value.
   * @param marketJson - JSON-serialized MarketContext whose quotes, curves, and other objects satisfy this calculation's documented requirements.
   * @param asOf - ISO-8601 valuation date used to select market inputs and date-dependent cashflows.
   * @param model - Pricing-model key selecting the valuation model implemented by the underlying Rust API.
   * @returns Returns the computed numeric result in the units described above.
   */
  volga(marketJson: string, asOf: string, model?: string | null): number;
}

/**
 * Construction and factory entry points for `FxInstrument` WebAssembly values.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const spot = new valuations.fx.FxSpot({
 *   id: "EURUSD-SPOT",
 *   base_currency: "EUR",
 *   quote_currency: "USD",
 *   settlement: "2025-01-17",
 *   spot_rate: 1.1,
 *   notional: { amount: "1000000", currency: "EUR" },
 *   attributes: {},
 * });
 * console.log(JSON.parse(spot.toJson()).instrument.type);
 * spot.free();
 * ```
 */
export interface FxInstrumentConstructor<T extends FxInstrument> {
  /**
   * Create a new `FxInstrument` WebAssembly value.
   * @param spec - Structured specification that defines the requested object or calculation.
   * @returns Returns the resulting `T` value or WebAssembly handle.
   */
  new (spec: FxInstrumentSpec): T;
  /**
   * Parse a `FxInstrument` value from canonical JSON.
   * @param json - JSON-serialized representation accepted by this API.
   * @returns Returns the resulting `T` value or WebAssembly handle.
   */
  fromJson(json: string): T;
}

/**
 * Namespaced TypeScript entry points for fx calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const Spot = valuations.fx.FxSpot;
 * const spot = new Spot({
 *   id: "EURUSD-SPOT",
 *   base_currency: "EUR",
 *   quote_currency: "USD",
 *   settlement: "2025-01-17",
 *   spot_rate: 1.1,
 *   notional: { amount: "1000000", currency: "EUR" },
 *   attributes: {},
 * });
 * console.log(spot.toJson());
 * spot.free();
 * ```
 */
export interface FxNamespace {
  /**
   * Fx spot exposed by this `Fx` value.
   */
  FxSpot: FxInstrumentConstructor<FxInstrument>;
  /**
   * Fx forward exposed by this `Fx` value.
   */
  FxForward: FxInstrumentConstructor<FxInstrument>;
  /**
   * Fx swap exposed by this `Fx` value.
   */
  FxSwap: FxInstrumentConstructor<FxInstrument>;
  /**
   * Ndf exposed by this `Fx` value.
   */
  Ndf: FxInstrumentConstructor<FxInstrument>;
  /**
   * Fx option exposed by this `Fx` value.
   */
  FxOption: FxInstrumentConstructor<FxOptionInstrument>;
  /**
   * Fx digital option exposed by this `Fx` value.
   */
  FxDigitalOption: FxInstrumentConstructor<FxDigitalOptionInstrument>;
  /**
   * Fx touch option exposed by this `Fx` value.
   */
  FxTouchOption: FxInstrumentConstructor<FxTouchOptionInstrument>;
  /**
   * Fx barrier option exposed by this `Fx` value.
   */
  FxBarrierOption: FxInstrumentConstructor<FxBarrierOptionInstrument>;
  /**
   * Fx variance swap exposed by this `Fx` value.
   */
  FxVarianceSwap: FxInstrumentConstructor<FxInstrument>;
  /**
   * Quanto option exposed by this `Fx` value.
   */
  QuantoOption: FxInstrumentConstructor<FxOptionInstrument>;
}

// --- SABR (Stochastic Alpha Beta Rho) volatility -------------------------

/**
 * SABR model parameters `(alpha, beta, nu, rho)` with optional `shift`.
 */
export interface SabrParameters extends WasmOwned {
  /**
   * SABR `alpha` (ATM volatility level).
   */
  readonly alpha: number;
  /**
   * SABR `beta` (backbone exponent).
   */
  readonly beta: number;
  /**
   * SABR `nu` (vol-of-vol).
   */
  readonly nu: number;
  /**
   * SABR `rho` (spot/vol correlation).
   */
  readonly rho: number;
  /**
   * Displacement applied for shifted SABR, if any.
   */
  readonly shift: number | undefined;
  /**
   * Whether a displacement (shift) is configured.
   * @returns Returns `true` when the documented condition is satisfied.
   */
  isShifted(): boolean;
}

/**
 * SABR model parameters `(alpha, beta, nu, rho)` with optional `shift`.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const params = new valuations.SabrParameters(0.2, 1.0, 0.3, -0.2);
 * console.log(params.alpha, params.rho);
 * params.free();
 * ```
 */
export interface SabrParametersConstructor {
  /**
   * Create the object from its inputs.
   * @returns Returns the resulting `SabrParameters` value or WebAssembly handle.
   * @param alpha - Positive SABR initial volatility scale parameter.
   * @param beta - SABR CEV elasticity parameter from 0 through 1.
   * @param nu - Positive SABR volatility-of-volatility parameter.
   * @param rho - Instantaneous correlation between the asset and variance shocks.
   * @param shift - Additive SABR rate shift applied to forward and strike before modelling.
   * @throws Error - Throws a JavaScript exception if `alpha` is not finite and positive, `beta` is outside `[0, 1]`, `nu` is negative or non-finite, `rho` is outside `[-1, 1]`, or a supplied `shift` is not finite and positive.
   */
  new (alpha: number, beta: number, nu: number, rho: number, shift?: number): SabrParameters;
  /**
   * Equity-standard defaults `(alpha=0.20, beta=1.0, nu=0.30, rho=-0.20)`.
   * @returns Returns the resulting `SabrParameters` value or WebAssembly handle.
   */
  equityDefault(): SabrParameters;
  /**
   * Rates-standard defaults `(alpha=0.02, beta=0.5, nu=0.30, rho=0.0)`.
   * @returns Returns the resulting `SabrParameters` value or WebAssembly handle.
   */
  ratesDefault(): SabrParameters;
}

/**
 * Hagan-2002 SABR volatility model.
 */
export interface SabrModel extends WasmOwned {
  /**
   * Black implied volatility for the given strike.
   * @returns Returns the computed numeric result in the units described above.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @throws Error - Throws a JavaScript exception if `t` is not positive, the forward or strike lies outside the selected shifted or unshifted SABR domain, or the Hagan expansion produces an undefined or non-finite volatility.
   */
  impliedVol(forward: number, strike: number, t: number): number;
  /**
   * Parameters used by this model.
   */
  readonly params: SabrParameters;
  /**
   * Whether the parameterization admits negative forwards.
   * @returns Returns `true` when the documented condition is satisfied.
   */
  supportsNegativeRates(): boolean;
}

/**
 * Hagan-2002 SABR volatility model.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const params = valuations.SabrParameters.equityDefault();
 * const model = new valuations.SabrModel(params);
 * console.log(model.impliedVol(100, 105, 1));
 * model.free();
 * params.free();
 * ```
 */
export interface SabrModelConstructor {
  /**
   * Create the object from its inputs.
   * @returns Returns the resulting `SabrModel` value or WebAssembly handle.
   * @param params - SABR parameter object containing alpha, beta, nu, rho, and optional shift.
   */
  new (params: SabrParameters): SabrModel;
}

/**
 * TypeScript view of the `SabrSmileArbitrageResult` WebAssembly value.
 */
export interface SabrSmileArbitrageResult {
  /**
   * Arbitrage free exposed by this `SabrSmileArbitrageResult` value.
   */
  arbitrage_free: boolean;
  /**
   * Butterfly violations exposed by this `SabrSmileArbitrageResult` value.
   */
  butterfly_violations: Array<{
    strike: number;
    butterfly_value: number;
    severity_pct: number;
  }>;
  /**
   * Monotonicity violations exposed by this `SabrSmileArbitrageResult` value.
   */
  monotonicity_violations: Array<{
    strike_low: number;
    strike_high: number;
    price_low: number;
    price_high: number;
  }>;
}

/**
 * Volatility smile generator for a fixed `(forward, t)` pair.
 */
export interface SabrSmile extends WasmOwned {
  /**
   * At-the-money implied volatility.
   * @returns Returns the computed numeric result in the units described above.
   * @throws Error - Throws a JavaScript exception if the smile's expiry or effective forward is outside the model domain, or the ATM calculation produces an invalid volatility.
   */
  atmVol(): number;
  /**
   * Black implied volatility for the given strike.
   * @returns Returns the computed numeric result in the units described above.
   * @param strike - Option strike price in the same price units as the underlying.
   * @throws Error - Throws a JavaScript exception if the smile's expiry, forward, or requested `strike` is outside the model domain, the Hagan expansion fails, or no volatility is returned for the strike.
   */
  impliedVol(strike: number): number;
  /**
   * Implied volatilities for a strike grid.
   * @returns Returns the resulting `number[]` collection in the documented order.
   * @param strikes - Option strikes at which to evaluate the SABR volatility smile.
   * @throws Error - Throws a JavaScript exception if the smile's expiry or forward, or any supplied strike, is outside the model domain, or the Hagan expansion produces an invalid volatility.
   */
  generateSmile(strikes: number[]): number[];
  /**
   * Butterfly + monotonicity arbitrage diagnostics.
   *
   * Returns a JSON object with `arbitrage_free`, `butterfly_violations`,
   * and `monotonicity_violations` arrays (snake_case keys matching the Rust
   * canonical fields and the Python binding).
   * @returns Returns the resulting `SabrSmileArbitrageResult` value or WebAssembly handle.
   * @param strikes - Ordered option strikes used to test the calibrated smile for static arbitrage.
   * @param r - Continuously compounded risk-free rate, expressed as a decimal.
   * @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
   * @throws Error - Throws a JavaScript exception if volatility generation fails for the stored smile and supplied strikes, or the diagnostics cannot be converted to a JavaScript value.
   */
  arbitrageDiagnostics(strikes: number[], r?: number, q?: number): SabrSmileArbitrageResult;
}

/**
 * Volatility smile generator for a fixed `(forward, t)` pair.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const params = valuations.SabrParameters.ratesDefault();
 * const smile = new valuations.SabrSmile(params, 0.03, 2);
 * console.log(smile.generateSmile([0.02, 0.03, 0.04]));
 * smile.free();
 * params.free();
 * ```
 */
export interface SabrSmileConstructor {
  /**
   * Create the object from its inputs.
   * @returns Returns the resulting `SabrSmile` value or WebAssembly handle.
   * @param params - SABR parameter object containing alpha, beta, nu, rho, and optional shift.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   */
  new (params: SabrParameters, forward: number, t: number): SabrSmile;
}

/**
 * SABR calibrator (Levenberg-Marquardt with beta fixed).
 */
export interface SabrCalibrator extends WasmOwned {
  /**
   * Return a copy of this calibrator with an overridden convergence
   * tolerance, preserving all other settings (e.g. the iteration cap from
   * `highPrecision`).
   * @returns Returns the resulting `SabrCalibrator` value or WebAssembly handle.
   * @param tolerance - Non-negative numerical convergence tolerance for the calibration optimizer.
   */
  withTolerance(tolerance: number): SabrCalibrator;
  /**
   * Calibrate `(alpha, nu, rho)` to market vols with `beta` fixed.
   * @returns Returns the resulting `SabrParameters` value or WebAssembly handle.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param strikes - Option strikes aligned one-for-one with market_vols.
   * @param marketVols - Market-implied annualized volatilities aligned one-for-one with strikes.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param beta - SABR CEV elasticity parameter held fixed during calibration.
   * @throws Error - Throws a JavaScript exception if the strike and volatility lengths differ, the quote arrays are empty, the SABR inputs or fitted parameters are invalid, or the calibration solver does not converge.
   */
  calibrate(
    forward: number,
    strikes: number[],
    marketVols: number[],
    t: number,
    beta: number
  ): SabrParameters;
  /**
   * Calibrate with automatic shift selection for negative-rate smiles.
   *
   * When the forward or any strike is negative, a shifted-SABR fit is
   * performed with an automatically chosen shift; otherwise this behaves
   * like `calibrate`.
   * @returns Returns the resulting `SabrParameters` value or WebAssembly handle.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param strikes - Option strikes aligned one-for-one with market_vols.
   * @param marketVols - Market-implied annualized volatilities aligned one-for-one with strikes.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param beta - SABR CEV elasticity parameter held fixed during calibration.
   * @throws Error - Throws a JavaScript exception if the strike and volatility lengths differ, the quote arrays are empty, the required shift exceeds the supported standardized ladder, the SABR inputs or fitted parameters are invalid, or the calibration solver does not converge.
   */
  calibrateAutoShift(
    forward: number,
    strikes: number[],
    marketVols: number[],
    t: number,
    beta: number
  ): SabrParameters;
}

/**
 * SABR calibrator (Levenberg-Marquardt with beta fixed).
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const calibrator = valuations.SabrCalibrator.highPrecision();
 * const tighter = calibrator.withTolerance(1e-10);
 * tighter.free();
 * calibrator.free();
 * ```
 */
export interface SabrCalibratorConstructor {
  /**
   * Create the object from its inputs.
   * @returns Returns the resulting `SabrCalibrator` value or WebAssembly handle.
   */
  new (): SabrCalibrator;
  /**
   * Calibrator preset with tighter convergence tolerances.
   * @returns Returns the resulting `SabrCalibrator` value or WebAssembly handle.
   */
  highPrecision(): SabrCalibrator;
}

/**
 * Namespaced TypeScript entry points for valuation credit calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const model = valuations.credit.mertonModelJson(100, 0.25, 60, 0.03);
 * console.log(valuations.credit.mertonDefaultProbability(model, 1));
 * ```
 */
export interface ValuationCreditNamespace {
  /**
   * Build a structural Merton model JSON payload.
   * @returns Returns the requested string representation or JSON payload.
   * @param assetValue - Current fair value of the firm's assets in monetary units.
   * @param assetVol - Annualized volatility of firm-asset returns, expressed as a decimal.
   * @param debtBarrier - Positive debt face value defining the structural-model default barrier.
   * @param riskFreeRate - Annualized risk-free rate expressed as a decimal, such as 0.05 for 5%.
   * @throws Error - Throws a JavaScript exception if `asset_value`, `asset_vol`, or `debt_barrier` is non-positive, or if the model cannot be serialized to JSON.
   */
  mertonModelJson(
    assetValue: number,
    assetVol: number,
    debtBarrier: number,
    riskFreeRate: number
  ): string;
  /**
   * Build a CreditGrades structural model JSON payload.
   * @returns Returns the requested string representation or JSON payload.
   * @param equityValue - Current market value of equity in the firm's monetary units.
   * @param equityVol - Annualized equity-return volatility expressed as a decimal.
   * @param totalDebt - Total debt face value in the firm's monetary units.
   * @param riskFreeRate - Annualized risk-free rate expressed as a decimal, such as 0.05 for 5%.
   * @param barrierUncertainty - Lognormal dispersion of the CreditGrades default barrier, not a generic uncertainty score.
   * @param meanRecovery - Mean recovery rate at default expressed as a fraction from 0 through 1.
   * @throws Error - Throws a JavaScript exception if CreditGrades or Merton model validation rejects the supplied equity, volatility, debt, barrier-uncertainty, or recovery inputs, or if the model cannot be serialized to JSON.
   */
  creditGradesModelJson(
    equityValue: number,
    equityVol: number,
    totalDebt: number,
    riskFreeRate: number,
    barrierUncertainty: number,
    meanRecovery: number
  ): string;
  /**
   * Compute structural default probability from model JSON.
   * @returns Returns the computed numeric result in the units described above.
   * @param modelJson - Serialized Merton structural-credit model produced by this API's model builder.
   * @param horizon - Forward-looking model horizon measured in years.
   * @throws Error - Throws a JavaScript exception if `model_json` is malformed or does not deserialize as a Merton model.
   */
  mertonDefaultProbability(modelJson: string, horizon: number): number;
  /**
   * Compute distance-to-default from a Merton model JSON payload.
   *
   * Distance-to-default is `ln(V/B)/(sigma*sqrt(T))` plus drift adjustments.
   * Lower values indicate higher default risk.
   * @returns Returns the computed numeric result in the units described above.
   * @param modelJson - Serialized Merton structural-credit model produced by this API's model builder.
   * @param horizon - Forward-looking model horizon measured in years.
   * @throws Error - Throws a JavaScript exception if `model_json` is malformed or does not deserialize as a Merton model.
   */
  mertonDistanceToDefault(modelJson: string, horizon: number): number;
  /**
   * Compute the implied credit spread (per year) from a Merton model JSON
   * payload, given a recovery rate. Matches the structural-model-implied
   * spread used to back into a hazard curve.
   * @returns Returns the computed numeric result in the units described above.
   * @param modelJson - Serialized Merton structural-credit model produced by this API's model builder.
   * @param horizon - Forward-looking model horizon measured in years.
   * @param recovery - Recovery rate at default expressed as a fraction of par from 0 through 1.
   * @throws Error - Throws a JavaScript exception if `model_json` is malformed or does not deserialize as a Merton model, `horizon` is non-finite or non-positive, or `recovery` is outside `[0, 1]`.
   */
  mertonImpliedSpread(modelJson: string, horizon: number, recovery: number): number;
  /**
   * Evaluate a `DynamicRecoverySpec` JSON payload at a given accreted
   * notional, returning the implied recovery rate. Result is clamped to
   * `[0, base_recovery]`.
   * @returns Returns the computed numeric result in the units described above.
   * @param specJson - Serialized DynamicRecoverySpec JSON defining the notional-to-recovery mapping.
   * @param notional - Signed trade notional in the instrument's native currency units.
   * @throws Error - Throws a JavaScript exception if `spec_json` is malformed or does not deserialize as a dynamic-recovery specification.
   */
  dynamicRecoveryAtNotional(specJson: string, notional: number): number;
  /**
   * Evaluate an `EndogenousHazardSpec` JSON payload at a given leverage
   * level, returning the implied hazard rate. Floored at 0.
   * @returns Returns the computed numeric result in the units described above.
   * @param specJson - Serialized EndogenousHazardSpec JSON defining the leverage-to-hazard mapping.
   * @param leverage - Debt-to-assets leverage ratio used by the structural credit model.
   * @throws Error - Throws a JavaScript exception if `spec_json` is malformed or does not deserialize as an endogenous-hazard specification.
   */
  endogenousHazardAtLeverage(specJson: string, leverage: number): number;
  /**
   * Convenience evaluator: hazard rate after a PIK accrual updates the
   * outstanding notional. Computes leverage = `accreted_notional / asset_value`
   * then evaluates the hazard mapping.
   * @returns Returns the computed numeric result in the units described above.
   * @param specJson - Serialized EndogenousHazardSpec JSON defining the leverage-to-hazard mapping.
   * @param accretedNotional - Outstanding notional after PIK accrual, in the debt's monetary units.
   * @param assetValue - Current fair value of the firm's assets in monetary units.
   * @throws Error - Throws a JavaScript exception if `spec_json` is malformed or does not deserialize as an endogenous-hazard specification.
   */
  endogenousHazardAfterPikAccrual(
    specJson: string,
    accretedNotional: number,
    assetValue: number
  ): number;
  /**
   * Build a constant dynamic-recovery spec JSON payload.
   * @returns Returns the requested string representation or JSON payload.
   * @param recovery - Recovery rate at default expressed as a fraction of par from 0 through 1.
   * @throws Error - Throws a JavaScript exception if `recovery` is outside `[0, 1]` or the specification cannot be serialized to JSON.
   */
  dynamicRecoveryConstantJson(recovery: number): string;
  /**
   * Build an endogenous hazard power-law spec JSON payload.
   * @returns Returns the requested string representation or JSON payload.
   * @param baseHazard - Reference annual default intensity used by the leverage-to-hazard mapping.
   * @param baseLeverage - Positive reference debt-to-assets leverage ratio for the hazard mapping.
   * @param exponent - Positive exponent controlling sensitivity in the documented power-law mapping.
   * @throws Error - Throws a JavaScript exception if `base_hazard` is negative, `base_leverage` is non-positive, or the specification cannot be serialized to JSON.
   */
  endogenousHazardPowerLawJson(baseHazard: number, baseLeverage: number, exponent: number): string;
  /**
   * Build a credit-state JSON payload for toggle-exercise decisions.
   *
   * Parameter order follows the canonical Rust `CreditState` field order
   * (and the Python binding): `hazardRate`, `distanceToDefault`, `leverage`,
   * `accretedNotional`, `couponDue`, `assetValue`.
   * @returns Returns the requested string representation or JSON payload.
   * @param hazardRate - Annualized instantaneous default intensity, expressed as a decimal.
   * @param distanceToDefault - Optional distance to default, measured as standard deviations from the default point.
   * @param leverage - Debt-to-assets leverage ratio used by the structural credit model.
   * @param accretedNotional - Outstanding notional after PIK accrual, in the debt's monetary units.
   * @param couponDue - Cash coupon amount due at the toggle decision date, in debt monetary units.
   * @param assetValue - Current fair value of the firm's assets in monetary units.
   * @throws Error - Throws a JavaScript exception if the credit state cannot be serialized to JSON.
   */
  creditStateJson(
    hazardRate: number,
    distanceToDefault: number | null | undefined,
    leverage: number,
    accretedNotional: number,
    couponDue: number,
    assetValue?: number | null
  ): string;
  /**
   * Build a threshold toggle-exercise model JSON payload.
   * @returns Returns the requested string representation or JSON payload.
   * @param variable - Credit-state variable: `"hazard_rate"`, `"distance_to_default"`, or `"leverage"`.
   * @param threshold - Threshold value in the units of the selected credit-state variable.
   * @param direction - Threshold comparison: `"above"` selects PIK above the level and `"below"` below it.
   * @throws Error - Throws a JavaScript exception if `variable` or `direction` is not a supported value, or if the model cannot be serialized to JSON.
   */
  toggleExerciseThresholdJson(
    variable: 'hazard_rate' | 'distance_to_default' | 'leverage',
    threshold: number,
    direction: 'above' | 'below'
  ): string;
  /**
   * Build an optimal toggle-exercise model JSON payload.
   *
   * `nested_paths` is the Monte-Carlo path count for the nested optimal-exercise
   * simulation. It is rejected if it exceeds `Number.MAX_SAFE_INTEGER` (`2^53-1`):
   * `usize` counts marshal across the wasm boundary as IEEE-754 doubles, so a
   * larger value would round silently rather than fail loudly.
   * @returns Returns the requested string representation or JSON payload.
   * @param nestedPaths - Number of nested Monte Carlo paths for continuation-value estimation; must fit JavaScript's safe integer range.
   * @param equityDiscountRate - Annual equity-holder discount rate used in the nested toggle decision.
   * @param assetVol - Annualized volatility of firm-asset returns, expressed as a decimal.
   * @param riskFreeRate - Annualized risk-free rate expressed as a decimal, such as 0.05 for 5%.
   * @param horizon - Forward-looking model horizon measured in years.
   * @throws Error - Throws a JavaScript exception if `nested_paths` exceeds JavaScript's safe integer range or the model cannot be serialized to JSON.
   */
  toggleExerciseOptimalJson(
    nestedPaths: number,
    equityDiscountRate: number,
    assetVol: number,
    riskFreeRate: number,
    horizon: number
  ): string;
}

/**
 * Namespaced TypeScript entry points for credit derivatives calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * const cds = JSON.parse(valuations.creditDerivatives.creditDefaultSwapExampleJson());
 * console.log(cds.instrument.type);
 * ```
 */
export interface CreditDerivativesNamespace {
  /**
   * Example tagged `CreditDefaultSwap` instrument JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if the example envelope cannot be serialized to JSON.
   */
  creditDefaultSwapExampleJson(): string;
  /**
   * Example tagged `CDSIndex` instrument JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if the example envelope cannot be serialized to JSON.
   */
  cdsIndexExampleJson(): string;
  /**
   * Example tagged `CDSTranche` instrument JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if the example envelope cannot be serialized to JSON.
   */
  cdsTrancheExampleJson(): string;
  /**
   * Example tagged `CDSOption` instrument JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if the example option cannot be constructed or its envelope cannot be serialized to JSON.
   */
  cdsOptionExampleJson(): string;
}

/**
 * Namespaced TypeScript entry points for valuations calculations and types.
 * @example
 * ```typescript
 * import init, { valuations } from "finstack-quant-wasm";
 * await init();
 * console.log(valuations.instruments.listStandardMetrics());
 * ```
 */
export interface ValuationsNamespace {
  /**
   * Pearson correlation coefficient.
   * @param x - Numeric observation series aligned one-for-one with the other series.
   * @param y - Numeric observation series aligned one-for-one with the other series.
   * @throws Error - Throws a JavaScript exception if `x` or `y` cannot be decoded as a numeric array.
   */
  correlation: CorrelationNamespace;
  /**
   * Structural credit models and toggle-exercise helpers.
   */
  credit: ValuationCreditNamespace;
  /**
   * CDS-family JSON wrappers and pricing helpers.
   */
  creditDerivatives: CreditDerivativesNamespace;
  /**
   * Direct FX instrument wrappers.
   */
  fx: FxNamespace;
  /**
   * Instrument JSON validation and pricing helpers.
   */
  instruments: ValuationInstrumentsNamespace;
  /**
   * Deserialize a `ValuationResult` from JSON and return the canonical JSON.
   *
   * Validates the input conforms to the `ValuationResult` schema.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical valuation-result JSON to validate and reserialize.
   * @throws Error - Throws a JavaScript exception if `json` is malformed or does not match the `ValuationResult` schema, or the canonical result cannot be serialized.
   */
  validateValuationResultJson(json: string): string;
  /**
   * Validate a calibration plan JSON and return the canonical (pretty-printed) form.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @param envelope - Calibration envelope containing the plan, market data, and optional prior market objects.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if `json` is malformed, its calibration schema marker is missing, malformed, or unsupported, static envelope validation fails, or the canonical envelope cannot be serialized.
   */
  validateCalibrationJson(envelope: CalibrationEnvelope | string): string;
  /**
   * Execute a `CalibrationEnvelope` and return the full `CalibrationResultEnvelope`.
   * Accepts either a typed object or a pre-serialized JSON string.
   * The canonical path for building a `MarketContext` from quotes — the resulting
   * `result.final_market` is a materialized state ready for `MarketContext::try_from`
   * (Rust) or `result.market` (Python).
   *
   * @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
   * @param envelope - Calibration envelope containing the plan, market data, and optional prior market objects.
   * @returns Returns the resulting `CalibrationResultEnvelope` value or WebAssembly handle.
   * @throws Error - Throws a JavaScript exception if `envelopeJson` is malformed or violates the calibration schema or static plan contract, market context construction or a calibration step fails, a solver does not converge, or the result envelope cannot be serialized.
   */
  calibrate(envelope: CalibrationEnvelope | string): CalibrationResultEnvelope;
  /**
   * Pre-flight envelope validation without invoking the solver.
   *
   * Returns a JSON-serialized `ValidationReport` listing every error found
   * plus the dependency graph. Microseconds.
   * @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
   * @param envelope - Calibration envelope containing the plan, market data, and optional prior market objects.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if `envelopeJson` is malformed, its schema marker is missing, malformed, or unsupported, the envelope structure is invalid, or the validation report cannot be serialized. Semantic findings are returned in the report rather than thrown.
   */
  dryRun(envelope: CalibrationEnvelope | string): string;
  /**
   * Returns the static dependency graph of a calibration plan as JSON.
   * @param envelope_json - CalibrationEnvelope JSON containing targets, parameters, bounds, and dependencies.
   * @param envelope - Calibration envelope containing the plan, market data, and optional prior market objects.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Throws a JavaScript exception if `envelopeJson` is malformed, its schema marker is missing, malformed, or unsupported, the envelope structure is invalid, or the dependency graph cannot be serialized.
   */
  dependencyGraphJson(envelope: CalibrationEnvelope | string): string;
  /**
   * Market exposed by this `Valuations` value.
   */
  Market: typeof Market;
  /**
   * Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.
   *
   * @example
   * ```javascript
   * import init, { valuations } from "finstack-quant-wasm";
   * await init();
   * const price = valuations.bsPrice(
   *   100,    // spot
   *   100,    // strike (ATM)
   *   0.05,   // r = 5%
   *   0.0,    // q = 0
   *   0.20,   // sigma = 20%
   *   1.0,    // 1 year
   *   true,   // call
   * );
   * // price ≈ 10.45
   * ```
   *
   * @param spot - Spot price of the underlying.
   * @param strike - Strike of the option.
   * @param r - Risk-free rate, **decimal** continuously compounded
   * @param q - Continuous dividend yield (or foreign rate for FX),
   * @param sigma - Annualized volatility, **decimal**
   * @param t - Time to expiry in **years**.
   * @param isCall - `true` for a call, `false` for a put.
   * @returns Per-unit option price.
   * @throws If the inputs produce a non-finite price (e.g. negative volatility).
   */
  bsPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    isCall: boolean
  ): number;
  /**
   * Black-Scholes / Garman-Kohlhagen Greeks as a `{delta, gamma, vega, theta, rho, rho_q}` object.
   *
   * @example
   * ```javascript
   * const g = valuations.bsGreeks(100, 100, 0.05, 0.0, 0.20, 1.0, true);
   * // g.delta ≈ 0.64, g.gamma ≈ 0.019, g.vega ≈ 0.38 (per 1% vol)
   * ```
   * @param spot - Spot price of the underlying.
   * @param strike - Strike of the option.
   * @param r - Risk-free rate, **decimal** continuously compounded.
   * @param q - Dividend yield (or foreign rate for FX), **decimal**
   * @param sigma - Annualized volatility, **decimal**.
   * @param t - Time to expiry in **years**.
   * @param isCall - `true` for a call, `false` for a put.
   * @param thetaDays - Day-count denominator for theta. Default `365`.
   * @returns Object `{ delta, gamma, vega, theta, rho, rho_q }` (snake_case keys
   * @throws If serialization to JS fails (should not happen on valid inputs).
   */
  bsGreeks(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    isCall: boolean,
    thetaDays?: number
  ): {
    delta: number;
    gamma: number;
    vega: number;
    theta: number;
    rho: number;
    rho_q: number;
  };
  /**
   * Solve for Black-Scholes / Garman-Kohlhagen implied volatility.
   *
   * @example
   * ```javascript
   * const iv = valuations.bsImpliedVol(100, 100, 0.05, 0.0, 1.0, 10.45, true);
   * // iv ≈ 0.20
   * ```
   * @param spot - Spot price of the underlying.
   * @param strike - Strike of the option.
   * @param r - Risk-free rate, **decimal** continuously compounded.
   * @param q - Dividend yield, **decimal** continuously compounded.
   * @param t - Time to expiry in **years**.
   * @param price - Observed option price (per unit).
   * @param isCall - `true` for a call, `false` for a put.
   * @returns Annualized implied volatility, **decimal** (e.g. `0.20`).
   * @throws If `price` is below intrinsic value, above the no-arbitrage
   */
  bsImpliedVol(
    spot: number,
    strike: number,
    r: number,
    q: number,
    t: number,
    price: number,
    isCall: boolean
  ): number;
  /**
   * Solve for Black-76 (forward-based) implied volatility.
   * @returns Returns the computed numeric result in the units described above.
   * @param forward - Forward price or rate in the same quote convention as the strike.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param df - Discount factor from valuation to expiry, expressed as a positive decimal.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param price - Price in the documented quote convention for this instrument.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @throws Error - Throws a JavaScript exception if an input is non-finite; `forward`, `strike`, `df`, or `price` is not positive; the price is not above intrinsic value or cannot be bracketed; or the implied-volatility solver does not converge. A non-positive `t` returns zero volatility.
   */
  black76ImpliedVol(
    forward: number,
    strike: number,
    df: number,
    t: number,
    price: number,
    isCall: boolean
  ): number;
  /**
   * Reiner-Rubinstein continuous-monitoring barrier call price.
   *
   * `direction` is `"up"` or `"down"`, `knock` is `"in"` or `"out"`.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param barrier - Continuously monitored barrier level in the same price units as spot.
   * @param r - Continuously compounded risk-free rate, expressed as a decimal.
   * @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
   * @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param direction - Barrier direction: `"up"` for an upper barrier or `"down"` for a lower barrier.
   * @param knock - Barrier activation: `"in"` for knock-in or `"out"` for knock-out.
   * @throws Error - Throws a JavaScript exception if `direction` or `knock` is unsupported, or the supplied model inputs produce a non-finite barrier price.
   */
  barrierCall(
    spot: number,
    strike: number,
    barrier: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    direction: 'up' | 'down',
    knock: 'in' | 'out'
  ): number;
  /**
   * Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param r - Continuously compounded risk-free rate, expressed as a decimal.
   * @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
   * @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param numFixings - Positive number of equally spaced averaging observations before expiry.
   * @param averaging - Asian averaging convention: `"arithmetic"` (default) or `"geometric"`.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @throws Error - Throws a JavaScript exception if `averaging` is not `"arithmetic"` or `"geometric"`, or the supplied model inputs produce a non-finite option price.
   */
  asianOptionPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    numFixings: number,
    averaging?: 'arithmetic' | 'geometric',
    isCall?: boolean
  ): number;
  /**
   * Conze-Viswanathan lookback option.
   *
   * `strike_type` is `"fixed"` (default) or `"floating"`. For `"floating"`,
   * `strike` is ignored and `extremum` is the observed min/max to date.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param r - Continuously compounded risk-free rate, expressed as a decimal.
   * @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
   * @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param extremum - Observed running minimum for a call or maximum for a put, in spot-price units.
   * @param strikeType - Lookback payoff convention: `"fixed"` (default) or `"floating"`.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @throws Error - Throws a JavaScript exception if `strikeType` is not `"fixed"` or `"floating"`, or the supplied model inputs produce a non-finite option price.
   */
  lookbackOptionPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    extremum: number,
    strikeType?: 'fixed' | 'floating',
    isCall?: boolean
  ): number;
  /**
   * Quanto option (FX-adjusted cross-currency) price in domestic currency.
   *
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param t - Time from the curve base date in years on the documented day-count basis.
   * @param rateDomestic - Domestic continuously compounded risk-free rate, expressed as a decimal.
   * @param rateForeign - Foreign continuously compounded risk-free rate, expressed as a decimal.
   * @param divYield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param volAsset - Annualized asset-price volatility expressed as a decimal.
   * @param volFx - Annualized FX-rate volatility expressed as a decimal.
   * @param correlation - Instantaneous correlation between the documented asset and FX-rate shocks, from -1 to 1.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @throws If the inputs produce a non-finite price.
   */
  quantoOptionPrice(
    spot: number,
    strike: number,
    t: number,
    rateDomestic: number,
    rateForeign: number,
    divYield: number,
    volAsset: number,
    volFx: number,
    correlation: number,
    isCall?: boolean
  ): number;
  /**
   * SABR parameters `(alpha, beta, nu, rho)` with optional `shift`.
   */
  SabrParameters: SabrParametersConstructor;
  /**
   * Hagan-2002 SABR volatility model.
   */
  SabrModel: SabrModelConstructor;
  /**
   * SABR smile generator for a fixed `(forward, t)` pair.
   */
  SabrSmile: SabrSmileConstructor;
  /**
   * Levenberg-Marquardt SABR calibrator (beta fixed).
   */
  SabrCalibrator: SabrCalibratorConstructor;
  /**
   * Price a European option under the Black-Scholes model using the COS method.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param dividend - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param vol - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param maturity - Time to option expiry in years.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @param nTerms - Optional positive number of COS expansion terms; omit to use the pricer default.
   * @throws Error - Throws a JavaScript exception if the model produces a degenerate or invalid COS truncation range, a non-finite characteristic-function value or forward moment, or a non-finite option price.
   */
  bsCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    vol: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /**
   * Price a European option under the Variance Gamma model using the COS method.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param dividend - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param theta - Variance-Gamma drift parameter controlling skew in log returns.
   * @param nu - Variance-Gamma variance-rate parameter; larger values increase tail thickness.
   * @param maturity - Time to option expiry in years.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @param nTerms - Optional positive number of COS expansion terms; omit to use the pricer default.
   * @throws Error - Throws a JavaScript exception if the model produces a degenerate or invalid COS truncation range, a non-finite characteristic-function value or forward moment, or a non-finite option price.
   */
  vgCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    sigma: number,
    theta: number,
    nu: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /**
   * Price a European option under Merton (1976) jump-diffusion using the COS method.
   * @returns Returns the computed numeric result in the units described above.
   * @param spot - Current spot price or exchange rate in the documented quote convention.
   * @param strike - Option strike price in the same price units as the underlying.
   * @param rate - Interest rate expressed as a decimal, such as 0.05 for 5%.
   * @param dividend - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
   * @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param muJump - Mean log jump size in the Merton jump-diffusion model.
   * @param sigmaJump - Standard deviation of log jump sizes in the Merton jump-diffusion model.
   * @param lambda - Annual jump-arrival intensity in the Merton jump-diffusion model.
   * @param maturity - Time to option expiry in years.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @param nTerms - Optional positive number of COS expansion terms; omit to use the pricer default.
   * @throws Error - Throws a JavaScript exception if the model produces a degenerate or invalid COS truncation range, a non-finite characteristic-function value or forward moment, or a non-finite option price.
   */
  mertonJumpCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    sigma: number,
    muJump: number,
    sigmaJump: number,
    lambda: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /**
   * Simulated TARN coupon profile along a deterministic floating-rate path.
   *
   * Returns a JSON object:
   * ```text
   * {
   *   "coupons_paid": number[],
   *   "cumulative":   number[],
   *   "redemption_index": number | null,
   *   "redeemed_early":   boolean
   * }
   * ```
   *
   * Each period's coupon is `max(fixed_rate - L_i, coupon_floor) * day_count_fraction`.
   * Payments accumulate in a
   * [`CumulativeCouponTracker`](finstack_quant_valuations::instruments::rates::exotics_shared::cumulative_coupon::CumulativeCouponTracker) configured with
   * `target_coupon`; once cumulative hits the target, the final coupon is
   * capped and the instrument is considered redeemed.
   * @returns Returns the result using the declared TypeScript shape.
   * @param fixedRate - Fixed coupon rate in decimal form before subtracting each floating fixing.
   * @param couponFloor - Minimum period coupon rate in decimal form after the TARN rate calculation.
   * @param floatingFixings - Ordered floating-rate fixings in decimal form, one for each coupon period.
   * @param targetCoupon - Cumulative coupon target, as a fraction of notional, that redeems the TARN.
   * @param dayCountFraction - Accrual year fraction applied to each coupon period.
   * @throws Error - Throws a JavaScript exception if `fixed_rate`, `coupon_floor`, `target_coupon`, `day_count_fraction`, or any fixing is non-finite; `coupon_floor` is negative; `target_coupon` or `day_count_fraction` is non-positive; or the result cannot be converted to a JavaScript object.
   */
  tarnCouponProfile(
    fixedRate: number,
    couponFloor: number,
    floatingFixings: number[],
    targetCoupon: number,
    dayCountFraction: number
  ): {
    coupons_paid: number[];
    cumulative: number[];
    redemption_index: number | null;
    redeemed_early: boolean;
  };
  /**
   * Snowball coupon schedule.
   *
   *   `c_i = clip(c_{i-1} + fixed_rate - L_i, floor, cap)` with `c_0 = initial_coupon`.
   * @returns Returns the resulting `number[]` collection in the documented order.
   * @param initialCoupon - Starting coupon rate before the first snowball update, in decimal form.
   * @param fixedRate - Fixed coupon rate in decimal form added at each snowball step.
   * @param floatingFixings - Ordered floating-rate fixings in decimal form, one for each coupon period.
   * @param floor - Minimum permitted coupon rate in decimal form.
   * @param cap - Maximum permitted coupon rate in decimal form.
   * @throws Error - Throws a JavaScript exception if `initial_coupon` or `floor` is negative; `initial_coupon`, `fixed_rate`, `floor`, or any fixing is non-finite; or `cap` is NaN or is not greater than `floor`.
   */
  snowballCouponProfile(
    initialCoupon: number,
    fixedRate: number,
    floatingFixings: number[],
    floor: number,
    cap: number
  ): number[];
  /**
   * Path-independent inverse-floater coupon schedule.
   * @returns Returns the resulting `number[]` collection in the documented order.
   * @param fixedRate - Fixed coupon rate in decimal form before the leveraged floating deduction.
   * @param floatingFixings - Ordered floating-rate fixings in decimal form, one for each coupon period.
   * @param floor - Minimum permitted coupon rate in decimal form.
   * @param cap - Maximum permitted coupon rate in decimal form.
   * @param leverage - Positive multiplier applied to each floating fixing in the inverse-floater coupon.
   * @throws Error - Throws a JavaScript exception if `floor` is negative; `fixed_rate`, `floor`, `leverage`, or any fixing is non-finite; `leverage` is non-positive; or `cap` is NaN or is not greater than `floor`.
   */
  inverseFloaterCouponProfile(
    fixedRate: number,
    floatingFixings: number[],
    floor: number,
    cap: number,
    leverage: number
  ): number[];
  /**
   * Intrinsic (undiscounted, unhedged) payoff of a CMS spread option.
   *
   * `call:  notional * max(long_cms - short_cms - strike, 0)`
   * `put:   notional * max(strike - (long_cms - short_cms), 0)`
   * @returns Returns the computed numeric result in the units described above.
   * @param longCms - Long-tenor CMS rate in decimal form.
   * @param shortCms - Short-tenor CMS rate in decimal form.
   * @param strike - CMS rate-spread strike in decimal form.
   * @param isCall - Whether to value a call (`true`) or put (`false`); defaults follow the callable's contract.
   * @param notional - Signed trade notional in the instrument's native currency units.
   * @throws Error - Throws a JavaScript exception if a CMS rate or `strike` is non-finite, or if `notional` is negative or non-finite.
   */
  cmsSpreadOptionIntrinsic(
    longCms: number,
    shortCms: number,
    strike: number,
    isCall: boolean,
    notional: number
  ): number;
  /**
   * Accrued coupon on a range-accrual leg over a set of observations.
   *
   * Counts the fraction of observations with a rate in the inclusive interval
   * `[lower, upper]` and scales by the period day-count fraction:
   *
   * `accrued = coupon_rate * day_count_fraction * (#in-range / #observations)`.
   *
   * The call provision is not applied here.
   * @returns Returns the computed numeric result in the units described above.
   * @param lower - Inclusive lower bound of the observed-rate range, in decimal form.
   * @param upper - Inclusive upper bound of the observed-rate range, in decimal form.
   * @param observations - Observed floating rates in decimal form for the accrual period.
   * @param couponRate - Contractual coupon rate in decimal form before range weighting.
   * @param dayCountFraction - Accrual year fraction for the coupon period.
   * @throws Error - Throws a JavaScript exception if the range bounds are non-finite or not strictly ordered; `observations` is empty or contains a non-finite value; or `coupon_rate` or `day_count_fraction` is negative or non-finite.
   */
  callableRangeAccrualAccrued(
    lower: number,
    upper: number,
    observations: number[],
    couponRate: number,
    dayCountFraction: number
  ): number;
}

/**
 * Namespaced TypeScript entry point for valuations APIs.
 */
export declare const valuations: ValuationsNamespace;

// --- attribution -----------------------------------------------------------

/**
 * Parameters for P&L attribution via [`attribute_pnl`].
 */
export interface AttributionParams extends WasmOwned {}

/**
 * Namespaced TypeScript entry points for attribution calculations and types.
 * @example
 * ```typescript
 * import init, { attribution } from "finstack-quant-wasm";
 * await init();
 * console.log(attribution.defaultWaterfallOrder());
 * ```
 */
export interface AttributionNamespace {
  /**
   * Parameters constructor emitted by wasm-bindgen for attribution calls.
   *
   * `configJson` may include `{ "execution_policy": "serial" }` when the host
   * already parallelizes attribution at the portfolio or batch level.
   */
  AttributionParams: new (
    instrumentJson: string,
    marketT0Json: string,
    marketT1Json: string,
    asOfT0: string,
    asOfT1: string,
    methodJson: string,
    configJson?: string,
    fullCrossAttribution?: boolean
  ) => AttributionParams;
  /**
   * Run P&L attribution for a single instrument.
   *
   * Accepts an [`AttributionParams`] struct with the instrument JSON, two market
   * snapshots, dates, and a method descriptor. Returns the `PnlAttribution`
   * result as JSON. `config_json` may include `"execution_policy": "serial"`
   * for hosts that already parallelize attribution at a higher level.
   * @returns Returns the requested string representation or JSON payload.
   * @param params - Fully specified AttributionParams object containing instrument, markets, dates, and method.
   * @throws Error - Rejects malformed instrument, market, method, or configuration JSON; invalid ISO attribution dates; instrument or market reconstruction, pricing, FX, rounding, metric, or method-specific attribution failures; a caught attribution panic; or failure to serialize the result.
   */
  attributePnl(params: AttributionParams): string;
  /**
   * Run attribution from a full JSON `AttributionEnvelope` and return JSON.
   *
   * Power-user variant for full envelope round-trip workflows.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - JSON-serialized AttributionParams specification to validate and execute.
   * @throws Error - Rejects malformed, schema-incompatible, or unsupported-version `spec_json`; instrument or market reconstruction, pricing, FX, rounding, metric, or method-specific attribution failures; a caught parse or execution panic; or failure to serialize the result envelope.
   */
  attributePnlFromSpec(specJson: string): string;
  /**
   * Validate an attribution specification JSON.
   *
   * Deserializes against the `AttributionEnvelope` schema, checks the
   * `schema` version tag (the same gate `execute` applies, so a payload that
   * validates here cannot later be rejected at execution), and returns the
   * canonical JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed, schema-incompatible, or unsupported-version `json`, or failure to serialize the canonical attribution envelope.
   */
  validateAttributionJson(json: string): string;
  /**
   * Return the default waterfall factor ordering as canonical snake-case values.
   * @returns Canonical snake-case factor names in the documented order.
   * @throws Error - Rejects if the default factor identifiers cannot be serialized to JavaScript.
   */
  defaultWaterfallOrder(): string[];
  /**
   * Return the default metric IDs used by metrics-based attribution.
   * @returns Returns the resulting `string[]` collection in the documented order.
   * @throws Error - Rejects if the default metric identifiers cannot be serialized to JavaScript.
   */
  defaultAttributionMetrics(): string[];
}

/**
 * Namespaced TypeScript entry point for attribution APIs.
 */
export declare const attribution: AttributionNamespace;

// --- statements ------------------------------------------------------------

/**
 * Namespaced TypeScript entry points for statements calculations and types.
 * @example
 * ```typescript
 * import init, { statements } from "finstack-quant-wasm";
 * await init();
 * console.log(statements.parseFormula("revenue - expenses"));
 * ```
 */
export interface StatementsNamespace {
  /**
   * Validate a `FinancialModelSpec` JSON string.
   *
   * Deserializes the input against the model schema, runs semantic validation,
   * and returns the canonical (re-serialized) JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, an empty or invalid period timeline, reserved node identifiers, incompatible node fields or value types, invalid formulas or dimensions, an invalid waterfall, or failure to serialize the normalized model.
   */
  validateFinancialModelJson(json: string): string;
  /**
   * Get the node identifiers from a model specification JSON.
   *
   * Returns a JS array of node ID strings in declaration order.
   * @returns Returns the resulting `string[]` collection in the documented order.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or if the node identifiers cannot be serialized to JavaScript.
   */
  modelNodeIds(json: string): string[];
  /**
   * Validate a `CheckSuiteSpec` JSON string.
   *
   * Deserializes the spec, re-serializes to canonical form, and
   * returns the JSON string. Useful for client-side validation.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or failure to serialize the decoded check-suite specification.
   */
  validateCheckSuiteSpec(json: string): string;
  /**
   * Validate a `CapitalStructureSpec` JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or failure to serialize the decoded capital-structure specification.
   */
  validateCapitalStructureSpec(json: string): string;
  /**
   * Validate a `WaterfallSpec` JSON string.
   *
   * Performs both serde deserialization and the waterfall's internal
   * consistency check (for example rejecting `Sweep` ordered after `Equity`
   * when an ECF sweep is configured).
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`; duplicate or inconsistent payment priorities; incomplete available-cash priorities; invalid PIK or ECF-sweep settings; or failure to serialize the validated waterfall.
   */
  validateWaterfallSpec(json: string): string;
  /**
   * Validate an `EcfSweepSpec` JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or failure to serialize the decoded ECF-sweep specification.
   */
  validateEcfSweepSpec(json: string): string;
  /**
   * Validate a `PikToggleSpec` JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @param json - Canonical JSON string defining the object to deserialize or normalize.
   * @throws Error - Rejects malformed or schema-incompatible `json`, or failure to serialize the decoded PIK-toggle specification.
   */
  validatePikToggleSpec(json: string): string;
  /**
   * Evaluate a `FinancialModelSpec` and return the `StatementResult` JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - JSON-serialized FinancialModelSpec to evaluate across its statement periods.
   * @throws Error - Rejects malformed `model_json`, model semantic failures, invalid formula or dependency graphs, missing evaluation inputs, unsupported capital-structure requirements, or failure to serialize the statement result.
   */
  evaluateModel(modelJson: string): string;
  /**
   * Evaluate a `FinancialModelSpec` against a `MarketContext` as of a given date.
   *
   * Required for capital-structure-aware models. The `as_of` argument is an
   * ISO 8601 date string (e.g. `"2025-01-15"`).
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - JSON-serialized FinancialModelSpec to evaluate across its statement periods.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Rejects malformed model or market JSON, model semantic failures, an invalid ISO `as_of` date, invalid formulas or dependencies, missing market data, or failure to serialize the statement result.
   */
  evaluateModelWithMarket(modelJson: string, marketJson: string, asOf: string): string;
  /**
   * Run Monte Carlo simulation on a financial model (JSON in/out).
   * @returns Canonical JSON containing percentile summaries and optional path data.
   * @param modelJson - Canonical JSON payload representing the statement model.
   * @param configJson - Canonical JSON payload representing the Monte Carlo configuration.
   * @throws Error - Rejects malformed model or configuration JSON, zero simulation paths, a model containing capital structure, model compilation or dependency failures, any path-evaluation failure, or failure to serialize the results.
   */
  runMonteCarlo(modelJson: string, configJson: string): string;
  /**
   * Parse a DSL formula and return a debug string for its AST.
   *
   * Useful for previewing expression structure in UI tooling before
   * committing a formula to a model.
   * @returns Returns the requested string representation or JSON payload.
   * @param formula - Financial-model formula string to parse into its canonical expression representation.
   * @throws Error - Rejects trailing tokens, malformed or incomplete syntax, or a formula that exceeds the parser's nesting or term limits.
   */
  parseFormula(formula: string): string;
  /**
   * Validate that a DSL formula parses and compiles successfully.
   *
   * Returns `true` when the formula is valid; throws a `FinstackError`
   * otherwise. This mirrors the Python `validate_formula` API.
   * @returns Returns `true` when the documented condition is satisfied.
   * @param formula - Financial-model formula string to parse and validate without evaluation.
   * @throws Error - Rejects any formula that cannot be parsed as one complete DSL expression or compiled because it contains an unsupported component, function, or operator form.
   */
  validateFormula(formula: string): boolean;
}

/**
 * Namespaced TypeScript entry point for statements APIs.
 */
export declare const statements: StatementsNamespace;

// --- statements_analytics -------------------------------------------------

/**
 * TypeScript view of the `GoalSeekResult` WebAssembly value.
 */
export interface GoalSeekResult {
  /**
   * Solved value exposed by this `GoalSeekResult` value.
   */
  solved_value: number;
  /**
   * Updated model json exposed by this `GoalSeekResult` value.
   */
  updated_model_json?: string;
}

/**
 * Namespaced TypeScript entry points for statements analytics calculations and types.
 * @example
 * ```typescript
 * import init, { statements_analytics } from "finstack-quant-wasm";
 * await init();
 * console.log(statements_analytics.wacc(0.6, 0.1, 0.4, 0.05, 0.25));
 * ```
 */
export interface StatementsAnalyticsNamespace {
  /**
   * Run a sensitivity analysis on a financial model.
   *
   * Accepts JSON strings for the model spec and sensitivity configuration,
   * evaluates all perturbation scenarios, and returns JSON results.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param configJson - Canonical JSON payload representing the config consumed by this API.
   * @throws Error - Rejects malformed model or configuration JSON, invalid sensitivity modes or parameter perturbations, missing model nodes or periods, model-evaluation failures, or failure to serialize the sensitivity result.
   */
  runSensitivity(modelJson: string, configJson: string): string;
  /**
   * Run a variance analysis comparing two evaluated statement results.
   *
   * Returns JSON-serialized variance report.
   * @returns Returns the requested string representation or JSON payload.
   * @param baseJson - Canonical JSON payload representing the base consumed by this API.
   * @param comparisonJson - Canonical JSON payload representing the comparison consumed by this API.
   * @param configJson - Canonical JSON payload representing the config consumed by this API.
   * @throws Error - Rejects malformed result or configuration JSON, empty metric or period selections, a requested value missing from either result, or failure to serialize the variance report.
   */
  runVariance(baseJson: string, comparisonJson: string, configJson: string): string;
  /**
   * Evaluate all scenarios in a scenario set against a base model.
   *
   * Returns a JSON object mapping scenario names to their statement results.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param scenarioSetJson - Canonical JSON payload representing the scenario set consumed by this API.
   * @throws Error - Rejects malformed model or scenario-set JSON, an empty scenario set, invalid parent chains, overrides of missing nodes, failure to evaluate any scenario, or failure to serialize the result map.
   */
  evaluateScenarioSet(modelJson: string, scenarioSetJson: string): string;
  /**
   * Compute forecast accuracy metrics (MAE, MAPE, RMSE).
   *
   * Takes two float arrays (actual, forecast) and returns a JSON object
   * with keys `mae`, `mape`, `rmse`, `n`.
   * @returns Returns the resulting `BacktestForecastMetricsJson` value or WebAssembly handle.
   * @param actual - Actual realized values aligned one-for-one with the forecast series.
   * @param forecast - Forecast values aligned one-for-one with the actual realized series.
   * @throws Error - Rejects inputs that cannot be decoded as numeric JavaScript arrays, arrays with unequal lengths, empty arrays, or metrics that cannot be serialized to JavaScript.
   */
  backtestForecast(actual: number[], forecast: number[]): BacktestForecastMetricsJson;
  /**
   * Generate tornado chart entries for a sensitivity result.
   * @returns Returns the requested string representation or JSON payload.
   * @param resultJson - Canonical JSON payload representing the result consumed by this API.
   * @param metricNode - Statement metric node identifier selected for the requested analysis.
   * @param period - Model period label for the requested statement value or calculation.
   * @throws Error - Rejects malformed `result_json`, an invalid optional `period` identifier, or failure to serialize the generated entries. A missing metric produces no entry rather than rejecting.
   */
  generateTornadoEntries(resultJson: string, metricNode: string, period?: string): string;
  /**
   * Find the driver value that makes a target node reach a target value.
   * @returns Returns the resulting `GoalSeekResult` value or WebAssembly handle.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param targetNode - Statement node identifier whose value is driven toward the target.
   * @param targetPeriod - Model period label in which the goal-seek target is evaluated.
   * @param targetValue - Numeric target value the goal-seek routine attempts to reach.
   * @param driverNode - Statement node identifier adjusted by the goal-seek routine.
   * @param driverPeriod - Model period label of the adjustable goal-seek driver.
   * @param updateModel - Whether to return the model with the solved driver value applied.
   * @param boundsLo - Lower numeric bound allowed for the goal-seek driver.
   * @param boundsHi - Upper numeric bound allowed for the goal-seek driver.
   * @throws Error - Rejects malformed `model_json`, invalid target or driver period identifiers, exactly one supplied bound, missing target or driver nodes or periods, non-finite or unordered bounds, model-evaluation or solver-convergence failures, or failure to serialize the result or updated model.
   */
  goalSeek(
    modelJson: string,
    targetNode: string,
    targetPeriod: string,
    targetValue: number,
    driverNode: string,
    driverPeriod: string,
    updateModel: boolean,
    boundsLo?: number | null,
    boundsHi?: number | null
  ): GoalSeekResult;
  /**
   * Rank the headline DCF assumptions by enterprise-value impact.
   *
   * The statement model is evaluated once; each shocked point re-runs only the
   * DCF. Returns JSON with the baseline enterprise value, tornado entries as
   * deltas versus that baseline sorted by descending absolute swing, and the
   * effective (possibly clamped) shock levels.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the financial model spec consumed by this API.
   * @param wacc - Baseline weighted average cost of capital in decimal form (0.10 = 10%).
   * @param terminalValueJson - Canonical JSON payload representing the terminal value spec, selecting whether growth or the exit multiple is shocked.
   * @param ufcfNode - Node identifier holding unlevered free cash flow for the forecast periods.
   * @param netDebtOverride - Optional flat net-debt amount used instead of the model-derived bridge.
   * @param waccSensitivityBump - Absolute shock applied to WACC and to the terminal growth rate, in decimal (0.01 = +/-100 bp).
   * @param waccDenominatorEpsilon - Minimum spread preserved between WACC and the terminal growth rate so 1/(wacc - g) stays defined, in decimal.
   * @param exitMultipleBump - Absolute shock applied to an exit multiple, in turns of the multiple (1.0 = +/-1.0x).
   * @throws Error - Rejects malformed model or terminal-value JSON, model-evaluation failures, a missing UFCF series or model currency, inconsistent WACC or terminal-value assumptions, missing bridge inputs, valuation failures, or failure to serialize the sensitivity result.
   */
  dcfSensitivity(
    modelJson: string,
    wacc: number,
    terminalValueJson: string,
    ufcfNode: string,
    netDebtOverride?: number | null,
    waccSensitivityBump?: number | null,
    waccDenominatorEpsilon?: number | null,
    exitMultipleBump?: number | null
  ): string;
  /**
   * Evaluate a leveraged-buyout transaction against a statement model.
   *
   * Entry enterprise value is priced at the model's first period, the sponsor
   * equity check is solved as the sources-and-uses residual, and exit proceeds
   * are the exit enterprise value less the modelled net debt at the exit
   * period. IRR is out of scope: pair the returned `exit_equity_proceeds` with
   * the equity outflow at close and call `portfolio.mwrXirr`.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the financial model spec consumed by this API.
   * @param entryMultiple - Entry valuation multiple applied to the entry metric (8.5 = 8.5x).
   * @param entryMetricNode - Node identifier supplying the entry valuation metric, read at the model's first period.
   * @param exitMultiple - Exit valuation multiple applied to the exit metric (9.5 = 9.5x).
   * @param exitMetricNode - Node identifier supplying the exit valuation metric, read at the exit period.
   * @param exitNetDebtNode - Node identifier supplying net debt outstanding at the exit period, where a modelled amortisation schedule lands.
   * @param exitPeriod - Model period label at which the sponsor exits, e.g. "2029".
   * @param sourcesJson - Canonical JSON array of funded debt tranches at close, each {"name", "amount"} in the model currency.
   * @param transactionFees - Transaction fees and expenses funded at close, in the model currency.
   * @throws Error - Rejects malformed model or tranche JSON, an invalid `exit_period`, model evaluation or lookup failures, a missing model currency or period, non-finite transaction inputs or model values, negative tranche amounts, a non-positive sponsor equity check, check-suite failures, or result serialization failure.
   */
  evaluateLbo(
    modelJson: string,
    entryMultiple: number,
    entryMetricNode: string,
    exitMultiple: number,
    exitMetricNode: string,
    exitNetDebtNode: string,
    exitPeriod: string,
    sourcesJson: string,
    transactionFees: number
  ): string;
  /**
   * Weighted-average cost of capital (WACC).
   *
   * Blends the required return on equity with the after-tax cost of debt:
   * `WACC = w_E * r_E + w_D * r_D * (1 - T)`.
   * @returns Returns the blended discount rate as a decimal fraction.
   * @param equityWeight - Equity share of total capital as a decimal fraction (0.6 = 60% equity-funded).
   * @param costOfEquity - Required return on equity in decimal form, typically from CAPM (0.115 = 11.5%).
   * @param debtWeight - Debt share of total capital as a decimal fraction; must sum with the equity weight to 1.0.
   * @param costOfDebt - Pre-tax marginal borrowing yield in decimal form, before the interest tax shield (0.06 = 6%).
   * @param taxRate - Marginal corporate tax rate as a decimal fraction in [0, 1] (0.25 = 25%).
   * @throws Error - Rejects any non-finite input, negative capital weights, weights that do not sum to one within tolerance, or a `tax_rate` outside `[0, 1]`.
   */
  wacc(
    equityWeight: number,
    costOfEquity: number,
    debtWeight: number,
    costOfDebt: number,
    taxRate: number
  ): number;
  /**
   * Trace dependencies for a node and return ASCII tree.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param nodeId - Stable node identifier used to select the required domain object.
   * @throws Error - Rejects malformed `model_json`, formulas or clauses whose dependencies cannot be parsed, unknown formula references, a missing `node_id` or reachable dependency, or a dependency cycle.
   */
  traceDependencies(modelJson: string, nodeId: string): string;
  /**
   * Explain a formula for a specific node and period (JSON in/out).
   * @returns Returns the resulting `FormulaExplanationJson` value or WebAssembly handle.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @param nodeId - Stable node identifier used to select the required domain object.
   * @param period - Model period label for the requested statement value or calculation.
   * @throws Error - Rejects malformed model or result JSON, an invalid `period` identifier, a missing model node or node-period result, an invalid formula used to build the breakdown, or failure to serialize the explanation to JavaScript.
   */
  explainFormula(
    modelJson: string,
    resultsJson: string,
    nodeId: string,
    period: string
  ): FormulaExplanationJson;
  /**
   * Explain a formula for a specific node and period as formatted text.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @param nodeId - Stable node identifier used to select the required domain object.
   * @param period - Model period label for the requested statement value or calculation.
   * @throws Error - Rejects malformed model or result JSON, an invalid `period` identifier, a missing model node or node-period result, or an invalid formula used to build the explanation breakdown.
   */
  explainFormulaText(
    modelJson: string,
    resultsJson: string,
    nodeId: string,
    period: string
  ): string;
  /**
   * Generate a P&L summary report as formatted text.
   * @returns Returns the requested string representation or JSON payload.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @param lineItems - Ordered statement line-item definitions included in the summary report.
   * @param periods - Ordered period labels or observations aligned with the supplied data.
   * @throws Error - Rejects malformed `results_json`, `line_items` or `periods` values that are not JavaScript string arrays, or any period string that is not a valid statement period identifier.
   */
  plSummaryReport(resultsJson: string, lineItems: string[], periods: string[]): string;
  /**
   * Generate a credit assessment report as formatted text.
   * @returns Returns the requested string representation or JSON payload.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Rejects malformed `results_json` or an `as_of` value that is not a valid statement period identifier.
   */
  creditAssessmentReport(resultsJson: string, asOf: string): string;
  /**
   * Compute a credit assessment from statement results (JSON in/out).
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Rejects malformed `results_json`, an `as_of` value that is not a valid statement period identifier, or failure to serialize the assessment.
   * @returns Returns the requested string representation or JSON payload.
   */
  creditAssessment(resultsJson: string, asOf: string): string;
  /**
   * Run checks from a suite spec against a model (JSON in/out).
   *
   * Evaluates the model, resolves the suite spec into runnable checks
   * (built-in **and** user-defined formula checks), and returns a JSON
   * check report.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param suiteSpecJson - Canonical JSON payload representing the suite spec consumed by this API.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @throws Error - Rejects malformed model, suite, or supplied result JSON; check-suite resolution failures; model-evaluation failures when results are omitted; missing nodes, incompatible data, or invalid check configuration during execution; or failure to serialize the report.
   */
  runChecks(modelJson: string, suiteSpecJson: string, resultsJson?: string | null): string;
  /**
   * Run three-statement checks using node mappings.
   *
   * Accepts a model and a mapping JSON, builds the appropriate check
   * suite, evaluates the model, runs the checks, and returns the report.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param mappingJson - Canonical JSON payload representing the mapping consumed by this API.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @throws Error - Rejects malformed model, mapping, or supplied result JSON; model-evaluation failures when results are omitted; missing mapped nodes, incompatible data, or invalid check configuration; or failure to serialize the report.
   */
  runThreeStatementChecks(
    modelJson: string,
    mappingJson: string,
    resultsJson?: string | null
  ): string;
  /**
   * Run credit underwriting checks using credit-specific mappings.
   * @returns Returns the requested string representation or JSON payload.
   * @param modelJson - Canonical JSON payload representing the model consumed by this API.
   * @param mappingJson - Canonical JSON payload representing the mapping consumed by this API.
   * @param resultsJson - Canonical JSON payload representing the results consumed by this API.
   * @throws Error - Rejects malformed model, mapping, or supplied result JSON; model-evaluation failures when results are omitted; missing mapped nodes, incompatible data, or invalid check configuration; or failure to serialize the report.
   */
  runCreditUnderwritingChecks(
    modelJson: string,
    mappingJson: string,
    resultsJson?: string | null
  ): string;
  /**
   * Render a check report as plain text.
   * @returns Returns the requested string representation or JSON payload.
   * @param reportJson - Canonical JSON payload representing the report consumed by this API.
   * @throws Error - Rejects `report_json` when it is malformed or incompatible with the check report schema.
   */
  renderCheckReportText(reportJson: string): string;
  /**
   * Render a check report as HTML.
   * @returns Returns the requested string representation or JSON payload.
   * @param reportJson - Canonical JSON payload representing the report consumed by this API.
   * @throws Error - Rejects `report_json` when it is malformed or incompatible with the check report schema.
   */
  renderCheckReportHtml(reportJson: string): string;
  // Comps — comparable company analysis
  /**
   * Percentile rank of `value` within `data` on a 0-1 scale.
   *
   * Returns `null` when `data` is empty rather than a synthetic 0.5.
   * @returns Returns the result using the declared TypeScript shape.
   * @param value - Subject-company metric value to rank against the peer sample.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Rejects when `data` is not a numeric JavaScript array or the finite rank cannot be serialized. Empty/non-finite peer data or a non-finite `value` return `null` rather than rejecting.
   */
  percentileRank(value: number, data: number[]): number | null;
  /**
   * Z-score of `value` within `data`.
   *
   * Returns `null` when fewer than two observations are provided or the
   * peer variance is zero, instead of a synthetic zero.
   * @returns Returns the result using the declared TypeScript shape.
   * @param value - Subject-company metric value to standardize against the peer sample.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Rejects when `data` is not a numeric JavaScript array or the computed score cannot be serialized. Insufficient data, zero variance, or a non-finite `value` return `null` rather than rejecting.
   */
  zScore(value: number, data: number[]): number | null;
  /**
   * Descriptive statistics over a peer distribution.
   *
   * Returns `null` (matching the other comps helpers) when `data` is empty.
   * @returns Returns the result using the declared TypeScript shape.
   * @param data - Non-empty numeric observation array used by the requested statistic.
   * @throws Error - Rejects when `data` is not a numeric JavaScript array or the statistics cannot be serialized. No finite observations return `null`.
   */
  peerStats(data: number[]): PeerStatsJson | null;
  /**
   * Single-factor OLS fit of `y` on `x` evaluated at the subject observation.
   * @returns Returns the result using the declared TypeScript shape.
   * @param xValues - Comparable-company independent-variable values aligned with y_values.
   * @param yValues - Comparable-company dependent-variable values aligned with x_values.
   * @param subjectX - Subject company's independent-variable value for the fitted regression.
   * @param subjectY - Subject company's observed dependent-variable value for relative-value comparison.
   * @throws Error - Rejects when `x_values` or `y_values` is not a numeric JavaScript array, or the regression result cannot be serialized. Fewer than three paired values or an unidentifiable fit returns `null`.
   */
  regressionFairValue(
    xValues: number[],
    yValues: number[],
    subjectX: number,
    subjectY: number
  ): RegressionResultJson | null;
  /**
   * Compute a canonical valuation multiple for a company-metric bag.
   * @returns Returns the result using the declared TypeScript shape.
   * @param companyMetrics - Company financial-metric object supplying numerator and denominator inputs.
   * @param multiple - Supported valuation multiple identifier, such as EV/EBITDA or P/E.
   * @throws Error - Rejects when `company_metrics` is not a string-to-number JavaScript object, `multiple` is not a supported canonical identifier, or the computed value cannot be serialized. Missing or non-finite inputs and non-positive denominators return `null`.
   */
  computeMultiple(companyMetrics: unknown, multiple: string): number | null;
  /**
   * Composite rich/cheap scoring across multiple dimensions.
   * @returns Returns the resulting `RelativeValueResultJson` value or WebAssembly handle.
   * @param peerSet - Comparable-company metric records used to score relative value.
   * @param dimensions - Metric dimensions and weights included in the relative-value score.
   * @throws Error - Rejects when `peer_set` or `dimensions` cannot be decoded into its declared schema, when no scoring dimensions are supplied, or when the result cannot be serialized to JavaScript.
   */
  scoreRelativeValue(peerSet: unknown, dimensions: unknown[]): RelativeValueResultJson;
}

/**
 * Namespaced TypeScript entry point for statements analytics APIs.
 */
export declare const statements_analytics: StatementsAnalyticsNamespace;

// --- portfolio -------------------------------------------------------------

/**
 * TypeScript view of the `ScenarioRevalueResult` WebAssembly value.
 */
export interface ScenarioRevalueResult {
  /**
   * Valuation exposed by this `ScenarioRevalueResult` value.
   */
  valuation: Record<string, unknown>;
  /**
   * Report exposed by this `ScenarioRevalueResult` value.
   */
  report: Record<string, unknown>;
}

/**
 * Scenario-attributable profit and loss together with the scenario
 * application report.
 */
export interface ScenarioPnlResult {
  /**
   * Profit-and-loss ladder: base-currency `total` plus a `by_position`
   * map of per-position base-currency amounts. Positions added or removed
   * by the scenario are zero-filled against the missing side, so
   * `by_position` always sums to `total`.
   */
  pnl: Record<string, unknown>;
  /**
   * Report exposed by this `ScenarioPnlResult` value.
   */
  report: Record<string, unknown>;
}

/**
 * Browser-native materialization input accepted without Node.js APIs.
 */
export type MaterializationBundleInput = string | Uint8Array;

/**
 * Typed error thrown when a persisted contract cannot be loaded.
 */
export interface ContractValidationError extends Error {
  /**
   * Stable public error class name.
   */
  name: 'ContractValidationError';
  /**
   * Typed Rust contract-error variant in snake_case.
   */
  kind:
    | 'unsupported_version'
    | 'missing_version'
    | 'malformed_schema'
    | 'limit_exceeded'
    | 'report'
    | 'core'
    | 'contract';
  /**
   * Structured diagnostics when `kind` is `"report"`.
   */
  report?: ValidationReport;
}

/**
 * Successful strict materialization result.
 */
export interface PortfolioMaterializationResult {
  /**
   * Reusable WebAssembly portfolio handle.
   */
  portfolio: Portfolio;
  /**
   * Structured counts, diagnostics, cache hits, and phase timings.
   */
  report: MaterializationReport;
}

/**
 * Reusable bounded cache of decoded content-addressed instruments.
 *
 * @example
 * ```typescript
 * const cache = new portfolio.InstrumentArtifactCache(5_000);
 * console.log(cache.size);
 * cache.free();
 * ```
 */
export declare class InstrumentArtifactCache {
  /**
   * Create an empty cache with explicit bounds.
   * @param capacity - Maximum retained entries; defaults to 4,096.
   */
  constructor(capacity?: number);
  /**
   * Number of decoded artifacts currently retained.
   * @returns A non-negative entry count.
   */
  readonly size: number;
  /**
   * Cumulative number of successful cache-miss decodes.
   * @returns A non-negative decode count.
   */
  readonly decodeCount: number;
  /** Release the underlying wasm heap allocation. Do not use this handle afterward. */
  free(): void;
}

/**
 * Typed handle to a built portfolio. Construct once via
 * `Portfolio.fromSpec` and reuse it across cashflow / valuation calls to
 * skip the per-call `PortfolioSpec` parse + rebuild cost.
 */
export declare class Portfolio {
  private constructor();
  /**
   * Build a runtime portfolio from a portable embedded specification.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @returns A reusable portfolio handle.
   * @throws Error - If the specification is malformed or violates portfolio invariants.
   */
  static fromSpec(specJson: string): Portfolio;
  /**
   * Build a runtime portfolio from one strict persisted materialization bundle.
   * @param bundle - Complete UTF-8 JSON as a browser string or `Uint8Array`.
   * @param cache - Optional reusable decoded-instrument cache.
   * @returns The reusable portfolio handle and structured load report.
   * @throws ContractValidationError - If the contract is malformed, unsupported, invalid, or exceeds resource limits.
   */
  static fromMaterialization(
    bundle: MaterializationBundleInput,
    cache?: InstrumentArtifactCache
  ): PortfolioMaterializationResult;
  /**
   * Validate a materialization bundle and return diagnostics for form UIs.
   * @param bundle - Complete UTF-8 JSON as a browser string or `Uint8Array`.
   * @param cache - Optional reusable decoded-instrument cache.
   * @returns On success, counters and validation-only phase timings; on contract failure, bounded diagnostics.
   * @throws ContractValidationError - If validation cannot produce a report.
   */
  static validateMaterializationJson(
    bundle: MaterializationBundleInput,
    cache?: InstrumentArtifactCache
  ): MaterializationReport | ValidationReport;
  /** Portfolio identifier. @returns Stable portfolio ID. */
  readonly id: string;
  /** ISO-8601 valuation date. @returns Portfolio as-of date. */
  readonly asOf: string;
  /** Reporting currency. @returns ISO-4217 base currency code. */
  readonly baseCcy: string;
  /** Return the number of positions. @returns Non-negative position count. */
  numPositions(): number;
  /** Serialize the portable portfolio specification. @returns Canonical JSON string. */
  toSpecJson(): string;
  /** Release the underlying wasm heap allocation. Do not use this handle after calling `free()`. */
  free(): void;
}

/**
 * Namespaced TypeScript entry points for portfolio calculations and types.
 * @example
 * ```typescript
 * import init, { portfolio } from "finstack-quant-wasm";
 * await init();
 * const cache = new portfolio.InstrumentArtifactCache(32);
 * console.log(cache.size);
 * cache.free();
 * ```
 */
export interface PortfolioNamespace {
  /**
   * Reusable bounded decoded-instrument cache constructor.
   */
  InstrumentArtifactCache: typeof InstrumentArtifactCache;
  /**
   * Typed handle for cached portfolio builds.
   */
  Portfolio: typeof Portfolio;
  /**
   * Parse and validate a portfolio specification from JSON.
   *
   * Returns the re-serialized canonical JSON form.
   * @returns Returns the requested string representation or JSON payload.
   * @param jsonStr - Canonical JSON string to validate, parse, or normalize for this API.
   * @throws Error - Throws a JavaScript exception if `jsonStr` is malformed or does not match the `PortfolioSpec` schema, or if the canonical form cannot be serialized.
   */
  parsePortfolioSpec(jsonStr: string): string;
  /**
   * Compute a single-period Brinson-Fachler attribution from sector JSON.
   *
   * Accepts a JSON array of `SectorPeriod` objects and returns a JSON
   * `BrinsonPeriodResult`.
   * @returns Returns the requested string representation or JSON payload.
   * @param sectorsJson - Canonical JSON payload representing the sectors consumed by this API.
   * @throws Error - Throws a JavaScript exception if `sectorsJson` is malformed, contains no sectors or a non-finite weight or return, portfolio or benchmark weights do not sum to one, or the result cannot be serialized.
   */
  brinsonFachler(sectorsJson: string): string;
  /**
   * Compute Carino-linked multi-period Brinson attribution from period JSON.
   *
   * Accepts a JSON array of periods, where each period is an array of
   * `SectorPeriod` objects, and returns a JSON `CarinoLinkedAttribution`.
   * @returns Returns the requested string representation or JSON payload.
   * @param periodsJson - Canonical JSON payload representing the periods consumed by this API.
   * @throws Error - Throws a JavaScript exception if `periodsJson` is malformed, any period fails Brinson validation, the sequence is empty or changes sector ordering, a period return is non-finite or at most `-1`, or the result cannot be serialized.
   */
  carinoLink(periodsJson: string): string;
  /**
   * Compute a single-period Campisi fixed-income attribution from JSON.
   *
   * Decomposes both sides into carry / treasury / spread / selection and
   * splits the active return into allocation plus four active component
   * effects (Campisi 2000). Returns a JSON `FiAttributionResult`.
   *
   * Every snapshot must use the quote-reproducing `z_spread` basis:
   * `spread_duration` is the canonical Z-spread duration, and `spread` plus
   * `delta_spread` are the matching Z-spread level and move. OAS, G-spread, or
   * discount-margin values are incompatible. The numeric JSON shape has no
   * metric IDs, so this boundary cannot detect mislabeled spread provenance.
   *
   * Throws when JSON is malformed or canonical Rust validation rejects empty
   * sides, non-finite values, invalid weights or period length, or a sector
   * present on either side has `|net sector weight| <= 1e-6 * gross absolute
   * sector weight`. Spread-basis provenance cannot be validated from numeric
   * JSON alone.
   * @returns Returns the requested string representation or JSON payload.
   * @param portfolioJson - Canonical JSON array of `FiPositionSnapshot` objects describing the portfolio side on the quote-reproducing Z-spread basis; weights must sum to 1.
   * @param benchmarkJson - Canonical JSON array of `FiPositionSnapshot` objects describing the benchmark side on the quote-reproducing Z-spread basis; weights must sum to 1.
   * @param configJson - Canonical JSON `FiAttributionConfig`; `period_years` is its only field, is required (no default), and unknown keys are rejected.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed; either side is empty; a value is non-finite; weights do not sum to one; `periodYears` is not finite and positive; a sector has a zero or near-zero net weight relative to gross weight; or the result cannot be serialized.
   */
  campisiAttribution(portfolioJson: string, benchmarkJson: string, configJson: string): string;
  /**
   * Carino-link already-computed single-period Campisi results.
   *
   * Binds Rust `campisi_carino_link`. Each period carries its own
   * already-applied `period_years`, so periods of *different* lengths (e.g.
   * act/365 calendar months) link correctly here; prefer this entry point
   * whenever the periods are not all the same length. Returns a JSON
   * `FiCarinoLinkedResult`.
   *
   * Throws if no periods are supplied, sector ordering differs, a consumed
   * top-level return/effect, per-sector linked effect, or sector `total_active`
   * is non-finite, `active_return` disagrees with the portfolio-minus-benchmark
   * return, a sector `total_active` disagrees with its five effects, sector
   * effects do not reconcile to their declared top-level totals, the five
   * totals do not reconcile to `active_return` within the overflow-safe
   * scaled-L1 tolerance, a reconciliation residual is non-finite, or a return
   * is outside the Carino domain.
   * @returns Returns the requested string representation or JSON payload.
   * @param periodsJson - Canonical JSON array of `FiAttributionResult` objects in chronological order, as returned by `campisiAttribution`.
   * @throws Error - Throws a JavaScript exception if `periodsJson` is malformed, the sequence is empty or changes sector ordering, a consumed value or reconciliation is non-finite or inconsistent, a return is at most `-1`, or the linked result cannot be serialized.
   */
  campisiCarinoLink(periodsJson: string): string;
  /**
   * Compute per-period Campisi attributions from snapshots and Carino-link them.
   *
   * Binds Rust `campisi_carino_link_from_snapshots`. One shared config — hence
   * one shared `period_years` — is applied to every period, so this entry point
   * is only correct for equal-length periods; use `campisiCarinoLink` for
   * unequal periods. Returns a JSON `FiCarinoLinkedResult`.
   * @returns Returns the requested string representation or JSON payload.
   * @param periodsJson - Canonical JSON array of `FiPeriodInput` objects, each holding `portfolio` and `benchmark` arrays of `FiPositionSnapshot`.
   * @param configJson - Canonical JSON `FiAttributionConfig` applied to every period; `period_years` is its only field and is required (no default).
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, any period fails Campisi attribution validation, the computed periods fail Carino linking validation, or the result cannot be serialized.
   */
  campisiCarinoLinkFromSnapshots(periodsJson: string, configJson: string): string;
  /**
   * Reconcile the five Campisi effect totals against the active return.
   *
   * Binds the Rust method `FiAttributionResult::reconciliation_check`. The
   * decomposition reconciles by construction (selection is the residual), so
   * this is a floating-point sanity gate rather than a model check; without it
   * callers must re-sum the five totals by hand. Returns a JSON
   * `FiReconciliationReport` with `total_residual`, `is_reconciled` and
   * `tolerance`.
   * @returns Returns the requested string representation or JSON payload.
   * @param resultJson - Canonical JSON `FiAttributionResult` as returned by `campisiAttribution`; unknown fields are rejected.
   * @param tolerance - Absolute reconciliation tolerance in return units; `1e-10` suits return-space values.
   * @throws Error - Throws a JavaScript exception if `resultJson` is malformed or does not match `FiAttributionResult`, or if the reconciliation report cannot be serialized.
   */
  campisiReconciliationCheck(resultJson: string, tolerance: number): string;
  /**
   * Build a duration-cell base-return table from a reference universe.
   *
   * Binds Rust `cell_returns_from_reference` (Dynkin, Hyman & Vankudre 1998,
   * Appendix B): buckets `referenceJson` into fixed-width duration cells and
   * averages each cell's member total returns, interpolating interior gaps
   * and flat-extrapolating leading/trailing gaps. Returns a JSON
   * `DurationCellTable`.
   * @returns Returns the requested string representation or JSON payload.
   * @param referenceJson - Canonical JSON array of `ReferenceReturn` objects (`duration`, `total_return`, both decimals with duration in years); must be non-empty.
   * @param baseLabel - Label identifying the resulting curve (e.g. `"UST"`), carried through to the output's `base_label` for policy visibility.
   * @param configJson - Canonical JSON `CellConfig`; `width` is its only field (cell width in years, finite and positive) and is required, with no default.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, the reference universe is empty or contains an invalid duration or return, the cell width is not finite and positive, labels collide, the grid exceeds its safety bound, or the result cannot be serialized.
   */
  cellReturnsFromReference(referenceJson: string, baseLabel: string, configJson: string): string;
  /**
   * Build a duration-cell base-return table from start/end discount curves.
   *
   * Binds Rust `cell_returns_from_curves`: each cell's base return is the
   * holding-period return of a hypothetical zero-coupon position bought at
   * the cell midpoint off `start` and revalued off `end` after
   * `horizonYears` have elapsed. Every resulting cell is observed, unlike
   * the reference-universe path in `cellReturnsFromReference`. Returns a
   * JSON `DurationCellTable`.
   * @returns Returns the requested string representation or JSON payload.
   * @param start - Discount curve observed at the start of the holding period.
   * @param end - Discount curve observed `horizonYears` later, at period end.
   * @param horizonYears - Length of the holding period, in years; must be finite and positive.
   * @param maxDuration - Upper bound of the duration grid, in years; must be finite and strictly greater than `horizonYears`.
   * @param baseLabel - Label identifying the base curve (e.g. `"UST"`, `"USD-SOFR"`), stamped into the result purely for policy visibility.
   * @param configJson - Canonical JSON `CellConfig`; `width` is its only field and is required, with no default.
   * @throws Error - Throws a JavaScript exception if `configJson` is malformed; the width, horizon, or maximum duration is invalid; a cell matures within the holding period; the grid is too large or has duplicate labels; a required discount factor is not finite and positive; or the result cannot be serialized.
   */
  cellReturnsFromCurves(
    start: DiscountCurve,
    end: DiscountCurve,
    horizonYears: number,
    maxDuration: number,
    baseLabel: string,
    configJson: string
  ): string;
  /**
   * Compute duration-matched credit excess returns against a base-return table.
   *
   * Binds Rust `excess_returns` (Dynkin, Hyman & Vankudre 1998, Appendix B):
   * each position's `duration` is matched to its duration cell in
   * `tableJson` and the position's excess return is `total_return -
   * cell.base_return`, the credit-specific component of performance
   * isolated from the general level/shape move of the base curve. Returns a
   * JSON `ExcessReturnResult` with per-position and portfolio-level totals.
   *
   * Throws if the table is empty or has empty/duplicate cell labels,
   * non-finite/negative/zero-width, non-ascending, or overlapping cells; any
   * position is invalid or falls in no cell (including a valid gap); or
   * position weights do not sum to one.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionsJson - Canonical JSON array of `ExcessReturnPosition` objects (`id`, `weight`, `duration`, `total_return`); weights must sum to 1.
   * @param tableJson - Canonical JSON `DurationCellTable`, as returned by `cellReturnsFromReference` or `cellReturnsFromCurves`.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, the cell table is invalid, a position is invalid or falls in no cell, position weights do not sum to one, or the result cannot be serialized.
   */
  excessReturns(positionsJson: string, tableJson: string): string;
  /**
   * Compute a single-period hierarchical duration-cell x sector grid attribution.
   *
   * Binds Rust `grid_attribution` (Dynkin, Hyman & Vankudre 1998, Appendix
   * A): decomposes active return into a per-cell curve (positioning)
   * effect, a within-cell sector allocation effect, and a
   * security-selection residual per (cell, sector). Returns a JSON
   * `GridAttributionResult` whose `total_curve`, `total_sector` and
   * `total_selection` sum to `active_return` to floating-point precision
   * for well-conditioned inputs; among accepted inputs, the reconciliation
   * residual grows the closer any bucket's net weight sits to the
   * near-zero-net-weight rejection boundary (see the Rust module docs for
   * measured magnitudes).
   * @returns Returns the requested string representation or JSON payload.
   * @param portfolioJson - Canonical JSON array of `GridPosition` objects (`cell`, `sector`, `weight`, `total_return`) for the portfolio side; weights must sum to 1.
   * @param benchmarkJson - Canonical JSON array of `GridPosition` objects for the benchmark side; same weight-sum requirement.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, a weight or return is non-finite, either side's weights do not sum to one, a cell or cell-sector bucket has a zero or near-zero net weight relative to gross weight, or the result cannot be serialized.
   */
  gridAttribution(portfolioJson: string, benchmarkJson: string): string;
  /**
   * Carino-link multi-period hierarchical grid attribution results.
   *
   * Binds Rust `grid_carino_link` (Carino 1999): applies Carino smoothing to
   * a chronological sequence of single-period `gridAttribution` results so
   * the three top-level effects (`linked_curve`, `linked_sector`,
   * `linked_selection`) sum exactly to the geometrically compounded active
   * return. Only the three top-level effects are linked; per-cell /
   * per-(cell, sector) multi-period linking is out of scope. Returns a JSON
   * `GridCarinoLinkedResult`.
   *
   * Throws if no periods are supplied; a consumed return or top-level effect
   * is non-finite; `active_return` disagrees with the portfolio-minus-
   * benchmark return; the three effect totals do not reconcile to
   * `active_return` within the overflow-safe scaled-L1 tolerance; a return-
   * identity or reconciliation residual is non-finite; or a return is outside
   * the Carino domain.
   * @returns Returns the requested string representation or JSON payload.
   * @param periodsJson - Canonical JSON array of `GridAttributionResult` objects, in chronological order, each the parsed output of `gridAttribution`.
   * @throws Error - Throws a JavaScript exception if `periodsJson` is malformed, the sequence is empty, a consumed value is non-finite or inconsistent, a return is at most `-1`, or the linked result cannot be serialized.
   */
  gridCarinoLink(periodsJson: string): string;
  /**
   * Compute Jeet-Partani (2023) factor-Brinson unified attribution.
   *
   * Binds Rust `factor_brinson_attribution`: generalizes classical
   * Brinson-Fachler allocation/selection to continuous factor exposures by
   * replacing the sector partition with a factor-exposure matrix and a
   * caller-supplied benchmark factor-return vector. Returns a JSON
   * `FactorBrinsonResult` with `allocation`, `selection`, and their
   * per-factor / per-asset breakdowns.
   * @returns Returns the requested string representation or JSON payload.
   * @param inputJson - Canonical JSON `FactorBrinsonInput` with `asset_ids`, `asset_returns`, `exposures` (row-major n_assets x n_factors), `factor_names`, `portfolio_weights` and `benchmark_weights`; each weight vector must sum to 1.
   * @param factorReturns - Caller-supplied benchmark factor returns `f_b` as a `number[]` or `Float64Array`, length `input.factor_names`; the `Float64Array` returned by `analytics.constrainedLeastSquares` can be passed directly.
   * @throws Error - Throws a JavaScript exception if `inputJson` is malformed; the asset or factor sets are empty; dimensions disagree; a value is non-finite; either weight vector does not sum to one; benchmark factor completeness is outside tolerance; or the result cannot be serialized.
   */
  factorBrinsonAttribution(inputJson: string, factorReturns: NumericArray): string;
  /**
   * Compute a Modified-Dietz TWRR sub-period return from period JSON.
   * @returns Returns the result using the declared TypeScript shape.
   * @param periodJson - Canonical JSON payload representing the period consumed by this API.
   * @throws Error - Throws a JavaScript exception if `periodJson` is malformed or does not match the expected period schema. Invalid financial inputs return `undefined`.
   */
  twrrModifiedDietz(periodJson: string): number | undefined;
  /**
   * Geometrically link TWRR sub-period returns from returns JSON.
   * @returns Returns the result using the declared TypeScript shape.
   * @param returnsJson - Canonical JSON payload representing the returns consumed by this API.
   * @param horizonYears - Return-linking horizon measured in years for annualization.
   * @throws Error - Throws a JavaScript exception if `returnsJson` is malformed or a defined linked result cannot be serialized. Invalid return series produce `undefined`.
   */
  twrrLinked(returnsJson: string, horizonYears: number): string | undefined;
  /**
   * Compute money-weighted return via XIRR from dated cashflow JSON.
   * @returns Returns the computed numeric result in the units described above.
   * @param cashflowsJson - Canonical JSON payload representing the cashflows consumed by this API.
   * @throws Error - Throws a JavaScript exception if `cashflowsJson` is malformed, contains an invalid date or insufficient cash flows for XIRR, or the numerical root cannot be found.
   */
  mwrXirr(cashflowsJson: string): number;
  /**
   * Build a runtime portfolio from a JSON spec, validate, and round-trip.
   *
   * Deserializes the spec, constructs the portfolio with live instruments,
   * validates structural invariants, then re-serializes for confirmation.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @throws Error - Throws a JavaScript exception if `specJson` is malformed or violates the portfolio schema, a position has an invalid quantity or instrument specification, portfolio validation fails, or the round-trip form cannot be serialized.
   */
  buildPortfolioFromSpec(specJson: string): string;
  /**
   * Extract the total portfolio value from a JSON result.
   * @returns Returns the computed numeric result in the units described above.
   * @param resultJson - Canonical JSON payload representing the result consumed by this API.
   * @throws Error - Throws a JavaScript exception if `resultJson` is malformed or does not match the `PortfolioResult` schema.
   */
  portfolioResultTotalValue(resultJson: string): number;
  /**
   * Extract a specific metric from a portfolio result JSON.
   *
   * Returns `undefined` (via `Option`) if the metric was not produced.
   * @returns Returns the result using the declared TypeScript shape.
   * @param resultJson - Canonical JSON payload representing the result consumed by this API.
   * @param metricId - Stable metric identifier used to select the required domain object.
   * @throws Error - Throws a JavaScript exception if `resultJson` is malformed or does not match the `PortfolioResult` schema. An absent `metricId` returns `undefined`.
   */
  portfolioResultGetMetric(resultJson: string, metricId: string): number | undefined;
  /**
   * Aggregate portfolio metrics from a valuation JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param valuationJson - Canonical JSON payload representing the valuation consumed by this API.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, `baseCurrency` or `asOf` is invalid, valuation currency or date metadata is inconsistent, a required FX conversion is unavailable or invalid, or the metrics cannot be serialized.
   * @param baseCcy - Base ccy string consumed by this operation.
   */
  aggregateMetrics(
    valuationJson: string,
    baseCcy: string,
    marketJson: string,
    asOf: string
  ): string;
  /**
   * Value a portfolio from its spec and market context.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param strictRisk - Whether unavailable risk metrics are treated as calculation errors.
   * @throws Error - Throws a JavaScript exception if the portfolio or market JSON is malformed, portfolio construction or valuation fails, strict risk calculation cannot produce a requested metric, a required FX conversion is unavailable, or the valuation cannot be serialized.
   */
  valuePortfolio(specJson: string, marketJson: string, strictRisk: boolean): string;
  /**
   * Value an already-built [`Portfolio`] handle. Skips the per-call
   * `PortfolioSpec` parse + `Portfolio::from_spec` rebuild that
   * [`value_portfolio`] performs; use this when sweeping market scenarios
   * against a fixed portfolio.
   * @returns Returns the requested string representation or JSON payload.
   * @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param strictRisk - Whether unavailable risk metrics are treated as calculation errors.
   * @throws Error - Throws a JavaScript exception if `marketJson` is malformed, portfolio valuation fails, strict risk calculation cannot produce a requested metric, a required FX conversion is unavailable, or the valuation cannot be serialized.
   */
  valuePortfolioBuilt(portfolio: Portfolio, marketJson: string, strictRisk: boolean): string;
  /**
   * Aggregate the full classified cashflow ladder for a portfolio.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @throws Error - Throws a JavaScript exception if the portfolio or market JSON is malformed, portfolio construction fails, monetary cash-flow aggregation overflows, or the aggregate cannot be serialized.
   */
  aggregateFullCashflows(specJson: string, marketJson: string): string;
  /**
   * Aggregate the full classified cashflow ladder for an already-built
   * [`Portfolio`] handle.
   *
   * Skips the per-call `PortfolioSpec` parse + `Portfolio::from_spec` rebuild.
   * For batched or chained workflows (repeated cashflow builds across market
   * scenarios on the same portfolio), this is the cheap path.
   * @returns Returns the requested string representation or JSON payload.
   * @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @throws Error - Throws a JavaScript exception if `marketJson` is malformed, monetary cash-flow aggregation overflows, or the aggregate cannot be serialized.
   */
  aggregateFullCashflowsBuilt(portfolio: Portfolio, marketJson: string): string;
  /**
   * Apply a scenario to a portfolio and revalue.
   *
   * Returns a JS object with structured `valuation` and `report` values.
   * @returns Returns the resulting `ScenarioRevalueResult` value or WebAssembly handle.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param scenarioJson - Canonical JSON payload representing the scenario consumed by this API.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @throws Error - Throws a JavaScript exception if the portfolio, scenario, or market JSON is malformed; portfolio construction, scenario application, or revaluation fails; or the structured result cannot be converted to a JavaScript value.
   */
  applyScenarioAndRevalue(
    specJson: string,
    scenarioJson: string,
    marketJson: string
  ): ScenarioRevalueResult;
  /**
   * Apply a scenario to an already-built [`Portfolio`] handle and revalue.
   * Returns a JS object with structured `valuation` and `report` values.
   * @returns Returns the resulting `ScenarioRevalueResult` value or WebAssembly handle.
   * @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
   * @param scenarioJson - Canonical JSON payload representing the scenario consumed by this API.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @throws Error - Throws a JavaScript exception if the scenario or market JSON is malformed, scenario application or portfolio revaluation fails, or the structured result cannot be converted to a JavaScript value.
   */
  applyScenarioAndRevalueBuilt(
    portfolio: Portfolio,
    scenarioJson: string,
    marketJson: string
  ): ScenarioRevalueResult;
  /**
   * Compute the profit and loss attributable to a scenario.
   *
   * Values the portfolio against the unshocked market and against the
   * scenario-shocked market, and returns a JS object with structured `pnl`
   * (base-currency `total` plus `by_position`) and `report` values.
   * @returns Returns the resulting `ScenarioPnlResult` value or WebAssembly handle.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param scenarioJson - Canonical JSON payload representing the scenario whose profit-and-loss impact is measured.
   * @param marketJson - Canonical market-context JSON supplying the unshocked curves, quotes, and FX data used for the base leg.
   * @throws Error - Throws a JavaScript exception if the portfolio, scenario, or market JSON is malformed; portfolio construction, scenario application, or either valuation fails; valuation currencies are inconsistent; or the structured result cannot be converted to JavaScript.
   */
  scenarioPnl(specJson: string, scenarioJson: string, marketJson: string): ScenarioPnlResult;
  /**
   * Compute the profit and loss attributable to a scenario for an
   * already-built [`Portfolio`] handle.
   *
   * Values the portfolio against the unshocked market and against the
   * scenario-shocked market, and returns a JS object with structured `pnl`
   * (base-currency `total` plus `by_position`) and `report` values. Positions
   * added or removed by the scenario are zero-filled against the missing side,
   * so the drill-down always sums to the total.
   * @returns Returns the resulting `ScenarioPnlResult` value or WebAssembly handle.
   * @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
   * @param scenarioJson - Canonical JSON payload representing the scenario whose profit-and-loss impact is measured.
   * @param marketJson - Canonical market-context JSON supplying the unshocked curves, quotes, and FX data used for the base leg.
   * @throws Error - Throws a JavaScript exception if the scenario or market JSON is malformed, scenario application or either valuation fails, valuation currencies are inconsistent, or the structured result cannot be converted to JavaScript.
   */
  scenarioPnlBuilt(
    portfolio: Portfolio,
    scenarioJson: string,
    marketJson: string
  ): ScenarioPnlResult;
  /**
   * Optimize portfolio weights using the LP-based optimizer.
   *
   * Accepts a `PortfolioOptimizationSpec` JSON (portfolio + objective +
   * constraints + options) and a `MarketContext` JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, the portfolio, objective, constraints, weighting, or missing-metric policy is invalid, a required market-dependent valuation fails, the solver cannot produce a result, or the result cannot be serialized.
   */
  optimizePortfolio(specJson: string, marketJson: string): string;
  /**
   * Replay a portfolio through dated market snapshots.
   *
   * Accepts a portfolio spec, an array of dated market snapshots, and a
   * replay configuration. Returns a JSON-serialized `ReplayResult`.
   * @returns Returns the requested string representation or JSON payload.
   * @param specJson - Canonical portfolio specification JSON defining positions, quantities, and base currency.
   * @param snapshotsJson - Canonical JSON payload representing the snapshots consumed by this API.
   * @param configJson - Canonical JSON payload representing the config consumed by this API.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed; the portfolio, replay configuration, or snapshot dates and ordering are invalid; valuation, attribution, or currency conversion fails; best-effort replay retains no step; or the result cannot be serialized.
   */
  replayPortfolio(specJson: string, snapshotsJson: string, configJson: string): string;
  /**
   * Decompose portfolio VaR into position contributions via parametric Euler
   * allocation. Inputs mirror the Python binding's signature.
   *
   * `covariance_json` must deserialize to an `n x n` row-major nested array.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionIdsJson - Canonical JSON payload representing the position ids consumed by this API.
   * @param weightsJson - Canonical JSON payload representing the weights consumed by this API.
   * @param covarianceJson - Canonical JSON payload representing the covariance consumed by this API.
   * @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed; identifier, weight, or covariance dimensions disagree; the covariance matrix is not finite, symmetric, and positive semidefinite; `confidence` is not finite and in `(0.5, 1)`; or the result cannot be serialized.
   */
  parametricVarDecomposition(
    positionIdsJson: string,
    weightsJson: string,
    covarianceJson: string,
    confidence: number
  ): string;
  /**
   * Decompose portfolio Expected Shortfall into position contributions via
   * parametric Euler allocation.
   *
   * Returns an ES-shaped JSON payload mirroring the Python
   * ``parametric_es_decomposition`` return value: a top-level
   * ``{portfolio_var, portfolio_es, confidence, n_positions, contributions}``
   * object whose ``contributions`` entries are
   * ``{position_id, component_es, marginal_es, pct_contribution}``.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionIdsJson - Canonical JSON payload representing the position ids consumed by this API.
   * @param weightsJson - Canonical JSON payload representing the weights consumed by this API.
   * @param covarianceJson - Canonical JSON payload representing the covariance consumed by this API.
   * @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed; identifier, weight, or covariance dimensions disagree; the covariance matrix is not finite, symmetric, and positive semidefinite; `confidence` is not finite and in `(0.5, 1)`; or the result cannot be serialized.
   */
  parametricEsDecomposition(
    positionIdsJson: string,
    weightsJson: string,
    covarianceJson: string,
    confidence: number
  ): string;
  /**
   * Decompose portfolio VaR/ES from per-position scenario P&Ls via historical
   * simulation.
   *
   * `position_pnls_json` is a nested array shaped `[n_positions][n_scenarios]`.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionIdsJson - Canonical JSON payload representing the position ids consumed by this API.
   * @param positionPnlsJson - Canonical JSON payload representing the position pnls consumed by this API.
   * @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed, position or scenario dimensions disagree, `confidence` is not finite and in `(0.5, 1)`, too few scenarios resolve the requested tail, a P-and-L value is non-finite, or the result cannot be serialized.
   */
  historicalVarDecomposition(
    positionIdsJson: string,
    positionPnlsJson: string,
    confidence: number
  ): string;
  /**
   * Evaluate a per-position risk budget against actual component VaRs.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionIdsJson - Canonical JSON payload representing the position ids consumed by this API.
   * @param actualVarJson - Canonical JSON payload representing the actual var consumed by this API.
   * @param targetVarPctJson - Canonical JSON payload representing the target var pct consumed by this API.
   * @param portfolioVar - Total portfolio VaR used to convert risk-budget shares into absolute amounts.
   * @param utilizationThreshold - Actual-to-target risk ratio that flags a budget breach.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed, actual or target arrays do not match the identifier count, non-empty target shares do not sum to one within tolerance, nonzero component risk is paired with zero `portfolioVar`, or the result cannot be serialized.
   */
  evaluateRiskBudget(
    positionIdsJson: string,
    actualVarJson: string,
    targetVarPctJson: string,
    portfolioVar: number,
    utilizationThreshold: number
  ): string;
  /**
   * Effective bid-ask spread via Roll (1984). Returns `undefined` when the
   * serial covariance is non-negative (Roll assumption violated) or inputs too short.
   * @returns Returns the result using the declared TypeScript shape.
   * @param returnsJson - Canonical JSON payload representing the returns consumed by this API.
   * @throws Error - Throws a JavaScript exception if `returnsJson` is malformed or does not contain a numeric array. Invalid estimator inputs return `undefined`.
   */
  rollEffectiveSpread(returnsJson: string): number | undefined;
  /**
   * Amihud (2002) illiquidity ratio from returns and volumes.
   * @returns Returns the result using the declared TypeScript shape.
   * @param returnsJson - Canonical JSON payload representing the returns consumed by this API.
   * @param volumesJson - Canonical JSON payload representing the volumes consumed by this API.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed or does not contain a numeric array. Invalid estimator inputs return `undefined`.
   */
  amihudIlliquidity(returnsJson: string, volumesJson: string): number | undefined;
  /**
   * Trading days required to liquidate at the given participation rate.
   * @returns Returns the computed numeric result in the units described above.
   * @param positionValue - Current position market value in the relevant currency units.
   * @param avgDailyVolume - Average daily trading volume in the same units as the position size.
   * @param participationRate - Maximum fraction of average daily volume used for execution.
   */
  daysToLiquidate(positionValue: number, avgDailyVolume: number, participationRate: number): number;
  /**
   * Classify a position into a liquidity tier from its days-to-liquidate.
   *
   * Uses the default `[1, 5, 20, 60]` trading-day thresholds. Returns one of
   * `"tier1" .. "tier5"`.
   * @returns Returns the requested string representation or JSON payload.
   * @param daysToLiquidate - Days to liquidate supplied to liquidity tier; follow the type and convention required by the surrounding API.
   */
  liquidityTier(daysToLiquidate: number): string;
  /**
   * Liquidity-adjusted VaR following Bangia, Diebold, Schuermann & Stroughair (1999).
   * Loss sign convention: `var` and `lvar` are non-positive.
   * @returns Returns the requested string representation or JSON payload.
   * @param spreadMean - Mean bid-ask spread in the quote units required by the liquidity model.
   * @param spreadVol - Volatility of the bid-ask spread in the liquidity model's units.
   * @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
   * @param positionValue - Current position market value in the relevant currency units.
   * @throws Error - Throws a JavaScript exception if `var` is non-finite or positive; either spread input is non-finite or negative; `confidence` is outside `(0, 1)`; `positionValue` is non-finite; or the result cannot be serialized.
   * @param varValue - Value-at-Risk level or estimate consumed by this calculation.
   */
  lvarBangia(
    varValue: number,
    spreadMean: number,
    spreadVol: number,
    confidence: number,
    positionValue: number
  ): string;
  /**
   * Almgren-Chriss (2001) market impact decomposition for a uniform execution.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionSize - Trade size in shares or notional units for the execution calculation.
   * @param avgDailyVolume - Average daily trading volume in the same units as the position size.
   * @param volatility - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
   * @param executionHorizonDays - Planned execution horizon measured in trading days.
   * @param permanentImpactCoef - Permanent market-impact coefficient in the execution-cost model.
   * @param temporaryImpactCoef - Temporary market-impact coefficient in the execution-cost model.
   * @param referencePrice - Optional reference price used to express execution impact in monetary units.
   * @throws Error - Throws a JavaScript exception if `positionSize` is non-finite; volume, volatility, or horizon is not finite and positive; an impact coefficient is outside its valid range; `referencePrice` is present but not finite and positive; impact calculation fails; or the result cannot be serialized.
   */
  almgrenChrissImpact(
    positionSize: number,
    avgDailyVolume: number,
    volatility: number,
    executionHorizonDays: number,
    permanentImpactCoef: number,
    temporaryImpactCoef: number,
    referencePrice?: number | null
  ): string;
  /**
   * Kyle (1985) linear price impact lambda estimated from observed volumes
   * and returns via the Amihud-ratio proxy. Returns `undefined` on invalid inputs.
   * @returns Returns the result using the declared TypeScript shape.
   * @param volumesJson - Canonical JSON payload representing the volumes consumed by this API.
   * @param returnsJson - Canonical JSON payload representing the returns consumed by this API.
   * @param referencePrice - Positive price per share or contract used to convert the return-space ratio into price-space lambda.
   * @throws Error - Throws a JavaScript exception if either JSON input is malformed or does not contain a numeric array. Invalid estimator inputs, including a non-positive or non-finite `referencePrice`, return `undefined`.
   */
  kyleLambda(volumesJson: string, returnsJson: string, referencePrice: number): number | undefined;
  /**
   * Compute first-order factor sensitivities and return the matrix as JSON.
   *
   * Accepts a JSON array of positions, a JSON array of `FactorDefinition`,
   * a `MarketContext` JSON, an ISO 8601 date, and an optional `BumpSizeConfig`
   * JSON.  Returns a JSON object with `position_ids`, `factor_ids`, and a
   * row-major `data` matrix.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionsJson - Canonical portfolio-positions JSON to bump and revalue.
   * @param factorsJson - Canonical factor-definition JSON identifying the market factors to shock.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param bumpConfigJson - Canonical bump-configuration JSON defining factor shock sizes and conventions.
   * @throws Error - Throws a JavaScript exception if `asOf` is not a valid ISO date; any JSON input is malformed; a factor definition or bump configuration is invalid or unsupported; bumping or repricing fails; or the sensitivity matrix cannot be serialized.
   */
  computeFactorSensitivities(
    positionsJson: string,
    factorsJson: string,
    marketJson: string,
    asOf: string,
    bumpConfigJson?: string
  ): string;
  /**
   * Compute first-order factor sensitivities using a pre-parsed [`Market`].
   *
   * Avoids reparsing market JSON for repeated factor analytics calls.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionsJson - Canonical portfolio-positions JSON to bump and revalue.
   * @param factorsJson - Canonical factor-definition JSON identifying the market factors to shock.
   * @param market - Market context or JSON payload supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param bumpConfigJson - Canonical bump-configuration JSON defining factor shock sizes and conventions.
   * @throws Error - Throws a JavaScript exception if `asOf` is not a valid ISO date; a position, factor, or bump-config JSON input is malformed; a factor definition is invalid or unsupported; bumping or repricing fails; or the sensitivity matrix cannot be serialized.
   */
  computeFactorSensitivitiesWithMarket(
    positionsJson: string,
    factorsJson: string,
    market: Market,
    asOf: string,
    bumpConfigJson?: string
  ): string;
  /**
   * Compute scenario P&L profiles via full repricing and return as JSON.
   *
   * Same position/factor/market inputs as `computeFactorSensitivities`, plus
   * an optional `n_scenario_points` integer.
   * @returns Returns the requested string representation or JSON payload.
   * @param positionsJson - Canonical portfolio-positions JSON to bump and revalue.
   * @param factorsJson - Canonical factor-definition JSON identifying the market factors to shock.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param bumpConfigJson - Canonical bump-configuration JSON defining factor shock sizes and conventions.
   * @param nScenarioPoints - Positive number of evenly spaced bump levels in each P-and-L profile.
   * @throws Error - Throws a JavaScript exception if `asOf` is not a valid ISO date; any JSON input is malformed; a factor, bump configuration, or scenario-point count is invalid or unsupported; bumping or repricing fails; or the profiles cannot be serialized.
   */
  computePnlProfiles(
    positionsJson: string,
    factorsJson: string,
    marketJson: string,
    asOf: string,
    bumpConfigJson?: string,
    nScenarioPoints?: number
  ): string;
  /**
   * Compute scenario P&L profiles using a pre-parsed [`Market`].
   * @returns Returns the requested string representation or JSON payload.
   * @param positionsJson - Canonical portfolio-positions JSON to bump and revalue.
   * @param factorsJson - Canonical factor-definition JSON identifying the market factors to shock.
   * @param market - Market context or JSON payload supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @param bumpConfigJson - Canonical bump-configuration JSON defining factor shock sizes and conventions.
   * @param nScenarioPoints - Positive number of evenly spaced bump levels in each P-and-L profile.
   * @throws Error - Throws a JavaScript exception if `asOf` is not a valid ISO date; a position, factor, or bump-config JSON input is malformed; a factor or scenario-point count is invalid or unsupported; bumping or repricing fails; or the profiles cannot be serialized.
   */
  computePnlProfilesWithMarket(
    positionsJson: string,
    factorsJson: string,
    market: Market,
    asOf: string,
    bumpConfigJson?: string,
    nScenarioPoints?: number
  ): string;
  /**
   * Decompose portfolio risk into factor and position contributions.
   *
   * Uses the parametric (covariance-based) Euler decomposition.  Accepts
   * a JSON sensitivity matrix (same schema as the output of
   * `computeFactorSensitivities`), a `FactorCovarianceMatrix` JSON, and an
   * optional `RiskMeasure` JSON.
   *
   * Returns a JSON object with `total_risk`, `measure`, `residual_risk`,
   * `factor_contributions` (array), and `position_factor_contributions` (array).
   * @returns Returns the requested string representation or JSON payload.
   * @param sensitivitiesJson - Canonical factor-sensitivity result JSON to decompose.
   * @param covarianceJson - Factor covariance-matrix JSON aligned with the supplied sensitivities.
   * @param riskMeasureJson - Risk-measure configuration JSON selecting the decomposition metric.
   * @throws Error - Throws a JavaScript exception if any JSON input is malformed; sensitivity dimensions or factor axes disagree; the covariance matrix or risk measure is invalid; decomposition produces invalid variance or another non-finite value; or the result cannot be converted to and serialized as JSON.
   */
  decomposeFactorRisk(
    sensitivitiesJson: string,
    covarianceJson: string,
    riskMeasureJson?: string
  ): string;
}

/**
 * Namespaced TypeScript entry point for portfolio APIs.
 */
export declare const portfolio: PortfolioNamespace;

// --- scenarios -------------------------------------------------------------

/**
 * TypeScript view of the `ScenarioWarning` WebAssembly value.
 */
export interface ScenarioWarning {
  /**
   * Kind exposed by this `ScenarioWarning` value.
   */
  kind: string;
  [key: string]: unknown;
}

/**
 * TypeScript view of the `ScenarioApplyResult` WebAssembly value.
 */
export interface ScenarioApplyResult {
  /**
   * Market json exposed by this `ScenarioApplyResult` value.
   */
  market_json: string;
  /**
   * Model json exposed by this `ScenarioApplyResult` value.
   */
  model_json: string;
  /**
   * Operations applied exposed by this `ScenarioApplyResult` value.
   */
  operations_applied: number;
  /**
   * User operations exposed by this `ScenarioApplyResult` value.
   */
  user_operations: number;
  /**
   * Expanded operations exposed by this `ScenarioApplyResult` value.
   */
  expanded_operations: number;
  /**
   * Warnings exposed by this `ScenarioApplyResult` value.
   */
  warnings: ScenarioWarning[];
}

/**
 * TypeScript view of the `ScenarioApplyMarketResult` WebAssembly value.
 */
export interface ScenarioApplyMarketResult {
  /**
   * Market json exposed by this `ScenarioApplyMarketResult` value.
   */
  market_json: string;
  /**
   * Operations applied exposed by this `ScenarioApplyMarketResult` value.
   */
  operations_applied: number;
  /**
   * User operations exposed by this `ScenarioApplyMarketResult` value.
   */
  user_operations: number;
  /**
   * Expanded operations exposed by this `ScenarioApplyMarketResult` value.
   */
  expanded_operations: number;
  /**
   * Warnings exposed by this `ScenarioApplyMarketResult` value.
   */
  warnings: ScenarioWarning[];
}

/**
 * Namespaced TypeScript entry points for scenarios calculations and types.
 * @example
 * ```typescript
 * import init, { scenarios } from "finstack-quant-wasm";
 * await init();
 * console.log(scenarios.listBuiltinTemplates());
 * ```
 */
export interface ScenariosNamespace {
  /**
   * Parse and validate a scenario specification from JSON.
   *
   * Returns the validated, re-serialized JSON.
   * @returns Returns the requested string representation or JSON payload.
   * @param jsonStr - Canonical JSON string to validate, parse, or normalize for this API.
   * @throws Error - Rejects malformed or schema-incompatible `json_str`, a blank scenario ID, multiple time-roll operations, invalid operation identifiers or numeric fields, variant-specific operation violations, or serialization failure.
   */
  parseScenarioSpec(jsonStr: string): string;
  /**
   * Compose multiple scenario specs (JSON array) into a single scenario.
   *
   * Specs are merged in priority order (lower number runs first).
   * @returns Returns the requested string representation or JSON payload.
   * @param specsJson - JSON array of validated ScenarioSpec objects to compose in priority order.
   * @throws Error - Rejects malformed or schema-incompatible `specs_json`, composition that contains more than one time-roll operation, or failure to serialize the composed specification.
   */
  composeScenarios(specsJson: string): string;
  /**
   * Validate a scenario specification JSON without executing it.
   *
   * Returns `true` if valid, throws on error.
   * @returns Returns `true` when the documented condition is satisfied.
   * @param jsonStr - Canonical JSON string to validate, parse, or normalize for this API.
   * @throws Error - Rejects malformed or schema-incompatible `json_str`, a blank scenario ID, multiple time-roll operations, invalid operation identifiers or numeric fields, or variant-specific operation violations.
   */
  validateScenarioSpec(jsonStr: string): boolean;
  /**
   * List all built-in template identifiers.
   *
   * Returns a JSON array of template ID strings.
   * @returns Returns the resulting `string[]` collection in the documented order.
   * @throws Error - Rejects if the embedded template registry cannot be parsed and validated, or if its template identifiers cannot be serialized to JavaScript.
   */
  listBuiltinTemplates(): string[];
  /**
   * Get metadata for all built-in templates as a JSON string.
   * @returns Returns the requested string representation or JSON payload.
   * @throws Error - Rejects if the embedded template registry cannot be parsed and validated, or if its metadata cannot be serialized to JSON.
   */
  listBuiltinTemplateMetadata(): string;
  /**
   * Build a scenario spec from a built-in template.
   *
   * Returns JSON-serialized `ScenarioSpec`.
   * @returns Returns the requested string representation or JSON payload.
   * @param templateId - Identifier of a built-in scenario template in the embedded registry.
   * @throws Error - Rejects a failure to load the embedded registry, an unknown `template_id`, a template whose resolved scenario fails validation, or failure to serialize the scenario.
   */
  buildFromTemplate(templateId: string): string;
  /**
   * List component IDs for a built-in composite template.
   *
   * Returns a JS array of component ID strings.
   * @returns Returns the resulting `string[]` collection in the documented order.
   * @param templateId - Identifier of a built-in scenario template in the embedded registry.
   * @throws Error - Rejects a failure to load the embedded registry, an unknown `template_id`, or component identifiers that cannot be serialized to JavaScript.
   */
  listTemplateComponents(templateId: string): string[];
  /**
   * Build a specific component from a built-in composite template.
   * @returns Returns the requested string representation or JSON payload.
   * @param templateId - Identifier of a built-in scenario template in the embedded registry.
   * @param componentId - Identifier of a component within the selected composite template.
   * @throws Error - Rejects a failure to load the embedded registry, an unknown `template_id` or `component_id`, a component scenario that fails validation, or failure to serialize the scenario.
   */
  buildTemplateComponent(templateId: string, componentId: string): string;
  /**
   * Build a scenario spec from fields.
   * @returns Returns the requested string representation or JSON payload.
   * @param id - Stable identifier used to name and retrieve the supplied domain object.
   * @param operationsJson - JSON array of scenario operation specifications in execution order.
   * @param name - Optional human-readable scenario name.
   * @param description - Optional human-readable description of the scenario purpose.
   * @param priority - Execution priority; lower values run earlier during composition.
   * @param resolutionMode - Optional hierarchy conflict policy:
   * @throws Error - Rejects malformed or schema-incompatible `operations_json`, an unsupported `resolution_mode`, a blank scenario ID, multiple time-roll operations, invalid operation identifiers or numeric fields, variant-specific operation violations, or failure to serialize the scenario. `"most_specific_wins"` (default) or `"cumulative"`.
   */
  buildScenarioSpec(
    id: string,
    operationsJson: string,
    name?: string,
    description?: string,
    priority?: number,
    resolutionMode?: 'most_specific_wins' | 'cumulative'
  ): string;
  /**
   * Apply a scenario to a market context and financial model.
   *
   * Returns a JSON object with `market_json`, `model_json`,
   * `operations_applied`, `user_operations`, `expanded_operations`,
   * `rounding_context` (active rounding-mode stamp), `time_roll` (a
   * `RollForwardReport`, only present when the scenario contained a
   * `time_roll_forward` operation), and `warnings`.
   *
   * This entry point supplies no instrument portfolio and no holiday calendar
   * to the engine: instrument-scoped operations (`instrument_price_pct_by_*`,
   * `instrument_spread_bp_by_*`, correlation shocks) are inert and produce a
   * warning, and `time_roll_forward` in `business_days` mode adjusts without
   * holiday information.
   * @returns Returns the resulting `ScenarioApplyResult` value or WebAssembly handle.
   * @param scenarioJson - JSON-serialized ScenarioSpec to validate and apply.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param modelJson - JSON-serialized FinancialModelSpec that scenario operations may mutate.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Rejects malformed scenario, market, or model JSON, an invalid ISO `as_of` date, an invalid scenario operation, missing market objects or hierarchy context, statement-model execution failures, failure to encode the mutated contexts, or failure to serialize the application envelope to JavaScript.
   */
  applyScenario(
    scenarioJson: string,
    marketJson: string,
    modelJson: string,
    asOf: string
  ): ScenarioApplyResult;
  /**
   * Apply a scenario to a market context only (no model mutations).
   *
   * Returns the same envelope shape as [`apply_scenario`] minus `model_json`;
   * the same caveats apply (no instrument portfolio, no holiday calendar).
   * @returns Returns the resulting `ScenarioApplyMarketResult` value or WebAssembly handle.
   * @param scenarioJson - JSON-serialized ScenarioSpec to validate and apply.
   * @param marketJson - Canonical market-context JSON supplying curves, quotes, and FX data.
   * @param asOf - ISO-8601 valuation date used to resolve date-dependent market data.
   * @throws Error - Rejects malformed scenario or market JSON, an invalid ISO `as_of` date, an invalid scenario operation, missing market objects or hierarchy context, failure to encode the mutated market, or failure to serialize the application envelope to JavaScript.
   */
  applyScenarioToMarket(
    scenarioJson: string,
    marketJson: string,
    asOf: string
  ): ScenarioApplyMarketResult;
  /**
   * Compute horizon total return under a scenario.
   *
   * Applies a scenario specification to project an instrument forward, then
   * decomposes the resulting P&L using factor-based attribution.
   *
   * @param instrumentJson - Canonical `finstack_quant.instrument/1` envelope.
   * @param marketJson - JSON-serialized `MarketContext`.
   * @param asOf - Valuation date (ISO 8601).
   * @param scenarioJson - JSON-serialized `ScenarioSpec`.
   * @param method - Attribution method: "parallel", "waterfall", "metrics_based", "taylor".
   * @param configJson - Optional FinstackConfig JSON for horizon analysis; omit to use defaults.
   * @param calendarId - Optional holiday calendar (e.g. "nyse", "target") used to
   * @returns JSON-serialized `HorizonResult`.
   * @throws Error - Rejects malformed instrument, market, scenario, or configuration JSON; an invalid ISO `as_of` date; an unsupported attribution `method`; an unknown `calendar_id`; invalid, unsupported, or unresolved scenario operations; missing market data; pricing or attribution failures; or failure to serialize the horizon result. business-day adjust `time_roll_forward` targets under `business_days` mode. Omit for a weekends-only calendar; unknown identifiers throw.
   */
  computeHorizonReturn(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    scenarioJson: string,
    method?: string,
    configJson?: string,
    calendarId?: string
  ): string;
}

/**
 * Namespaced TypeScript entry point for scenarios APIs.
 */
export declare const scenarios: ScenariosNamespace;
