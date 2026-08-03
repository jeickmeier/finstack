/**
 * Legacy namespace fixture.
 * @example
 * ```typescript
 * import init, { math } from "finstack-quant-wasm";
 * await init();
 * const api: MathNamespace = math;
 * void api;
 * ```
 */
export interface MathNamespace {
  /**
   * Calculate a numeric result.
   * @param value - Numeric input.
   * @returns The numeric result.
   * @throws Error - Thrown when supplied values are malformed, violate the documented constraints, or the underlying calculation cannot complete.
   */
  calculate(value: number): number;
}

/**
 * Legacy constructor fixture.
 * @example
 * ```typescript
 * import init, { core } from "finstack-quant-wasm";
 * await init();
 * const factory: CalculatorConstructor = core.Calculator;
 * void factory;
 * ```
 */
export interface CalculatorConstructor {
  /** Create a calculator value without caller-supplied inputs. */
  new (): Calculator;
}

/** Calculator handle used by the legacy fixture. */
export interface Calculator {}

/**
 * Legacy package-root function fixture.
 * @example
 * ```typescript
 * import init, { calculate } from "finstack-quant-wasm";
 * await init();
 * // Supply the documented arguments to calculate(...) for your use case.
 * void calculate;
 * ```
 * @param value - Numeric input.
 * @returns The numeric result.
 * @throws Error - Thrown when `value` is not finite.
 */
export function calculate(value: number): number;
