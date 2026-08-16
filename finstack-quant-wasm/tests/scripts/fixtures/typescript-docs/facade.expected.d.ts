/**
 * Add values through the package-root facade.
 * @example
 * ```typescript
 * const total = addValues(2, 3);
 * console.log(total);
 * ```
 * @param leftValue - Left addend expressed in currency units.
 * @param rightValue - Right addend expressed in the same currency units.
 * @returns Returns the sum in currency units.
 * @throws Error - Throws a JavaScript exception if either addend is non-finite.
 */
export function addValues(leftValue: number, rightValue: number): number;

/**
 * Calculator factory exposed by the package facade.
 * @example
 * ```typescript
 * const calculator = Calculator.fromJson('{"precision": 2}');
 * console.log(calculator);
 * ```
 */
export interface CalculatorConstructor {
  /**
   * Parse a calculator.
   * @param json - Canonical calculator JSON.
   * @returns Returns the restored calculator.
   * @throws Error - Throws a JavaScript exception if `json` is malformed.
   */
  fromJson(json: string): Calculator;
}

/**
 * A calculator handle used by the fixture.
 */
export interface Calculator {}

export declare class ReportHandle {
  /**
   * Serialize this report to JSON.
   * @returns Canonical JSON string for the report.
   * @throws Error - Throws a JavaScript exception if serialization fails.
   */
  toJson(): string;
}

/**
 * Scale a decimal interest rate across a tenor.
 * @example
 * ```typescript
 * const scaled = scaleRate(0.05, 2);
 * console.log(scaled);
 * ```
 * @param rate - Annualized rate as a decimal (e.g. `0.05` for 5%).
 * @param tenor - Time horizon in years on an ACT/365 basis.
 * @returns Rate times tenor, as a decimal year-fraction product.
 * @throws If `rate` or `tenor` is non-finite or `tenor` is negative.
 */
export function scaleRate(rate: number, tenor: number): number;
