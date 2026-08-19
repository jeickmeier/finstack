import * as wasm from '../../pkg/finstack_quant_wasm.js';

const json = (value) => (typeof value === 'string' ? value : JSON.stringify(value));

/**
 * Composite-instrument construction, decomposition, execution, and history.
 *
 * Pricing uses frozen quantities. Only `initialize` and `rebalance` calculate
 * a new state. Period return is `pnl / capital`; `return_index` starts at 100.
 * Close-effective rebalances report pre-trade P&L, then open the next interval
 * at the post-trade financed value. There is no `initializeFixed` export.
 */
export const composite = {
  /**
   * Resolve a bare specification into a canonical instrument envelope and trades.
   *
   * Fixed-quantity specs do not require `history`. Volatility weighting requires
   * strictly increasing observations that end on `asOf`.
   */
  initialize: (spec, market, asOf, history = undefined) => {
    return wasm.initializeComposite(
      json(spec),
      json(market),
      asOf,
      history === undefined ? undefined : json(history)
    );
  },
  /**
   * Return a distinct resolved state and primitive deltas from an explicit rebalance.
   *
   * The supplied instrument is not mutated. Volatility history must end on `asOf`.
   */
  rebalance: (instrument, market, asOf, history = undefined) => {
    return wasm.rebalanceComposite(
      json(instrument),
      json(market),
      asOf,
      history === undefined ? undefined : json(history)
    );
  },
  /**
   * Price frozen primitive paths and return net/gross value and additive risk.
   *
   * Non-additive metrics are rejected. Amounts use the reporting currency on `asOf`.
   */
  primitiveExposures: (instrument, market, asOf, metrics = undefined) => {
    return wasm.compositePrimitiveExposures(json(instrument), json(market), asOf, metrics);
  },
  /**
   * Flatten target holdings or a state transition into primitive execution deltas.
   *
   * Omit `previous` to emit establishment trades for the current frozen state.
   */
  executionTrades: (instrument, previous = undefined) => {
    return wasm.compositeExecutionTrades(
      json(instrument),
      previous === undefined ? undefined : json(previous)
    );
  },
  /**
   * Initialize at the first snapshot and return dated P&L, return, index, exposure, and trade rows.
   *
   * Warmup feeds weighting only. The first row has `return_index = 100` and zero P&L.
   * Scheduled rebalances are close-effective.
   */
  historyFromSpec: (spec, observations, warmup = undefined, metrics = undefined) => {
    return wasm.compositeHistoryFromSpec(
      json(spec),
      json(observations),
      warmup === undefined ? undefined : json(warmup),
      metrics
    );
  },
  /**
   * Run dated history from an already-resolved immutable state.
   *
   * Period return is `pnl / capital`. The initial effective date must be on or
   * before the first observation.
   */
  history: (instrument, observations, metrics = undefined) => {
    return wasm.compositeHistory(json(instrument), json(observations), metrics);
  },
};
