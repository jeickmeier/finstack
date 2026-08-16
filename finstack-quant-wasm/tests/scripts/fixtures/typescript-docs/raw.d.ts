/**
 * Add two finite values.
 *
 * # Arguments
 *
 * * `left_value` - Left addend expressed in currency units.
 * * `right_value` - Right addend expressed in the same currency units.
 *
 * # Returns
 *
 * Returns the sum in currency units.
 *
 * # Errors
 *
 * Throws a JavaScript exception if either addend is non-finite.
 */
export function addValues(left_value: number, right_value: number): number;

export class Calculator {
  private constructor();

  /**
   * Restore a calculator from JSON.
   *
   * # Arguments
   *
   * * `json` - Canonical calculator JSON.
   *
   * # Returns
   *
   * Returns the restored calculator.
   *
   * # Errors
   *
   * Throws a JavaScript exception if `json` is malformed.
   */
  static fromJson(json: string): Calculator;
}

export class ReportHandle {
  /**
   * Serialize this report to canonical JSON.
   *
   * # Returns
   *
   * Canonical JSON string for the report.
   *
   * # Errors
   *
   * Throws a JavaScript exception if serialization fails.
   */
  toJson(): string;
}

/**
 * Scale a decimal interest rate across a tenor.
 *
 * @param rate - Annualized rate as a decimal
 * (e.g. `0.05` for 5%).
 * @param tenor - Time horizon in years on an ACT/365 basis.
 * @returns Rate times tenor, as a decimal year-fraction product.
 * @throws If `rate` or `tenor` is non-finite
 * or `tenor` is negative.
 */
export function scaleRate(rate: number, tenor: number): number;
