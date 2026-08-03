/**
 * Add values through the package-root facade.
 * @example
 * ```typescript
 * const total = addValues(2, 3);
 * console.log(total);
 * ```
 * @param leftValue - Stale left description.
 * @param rightValue - Stale right description.
 * @returns Stale return description.
 * @throws Error - Thrown when supplied values are malformed, violate the documented constraints, or the underlying calculation cannot complete.
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
   * @param json - Stale JSON description.
   * @returns Stale calculator description.
   * @throws Error - Thrown when supplied values are malformed, violate the documented constraints, or the underlying calculation cannot complete.
   */
  fromJson(json: string): Calculator;
}

/** A calculator handle used by the fixture. */
export interface Calculator {}
