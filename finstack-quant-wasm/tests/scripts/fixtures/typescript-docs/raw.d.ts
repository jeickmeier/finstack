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
