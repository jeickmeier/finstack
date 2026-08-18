import * as wasm from '../../pkg/finstack_quant_wasm.js';

const json = (value) => (typeof value === 'string' ? value : JSON.stringify(value));

/** Composite-instrument construction, decomposition, execution, and history. */
export const composite = {
  /** Resolve a bare composite specification into a canonical instrument envelope and trades. */
  initialize: (spec, market, asOf, history = undefined) => {
    return wasm.initializeComposite(
      json(spec),
      json(market),
      asOf,
      history === undefined ? undefined : json(history)
    );
  },
  /** Return a distinct resolved state and primitive deltas from an explicit rebalance. */
  rebalance: (instrument, market, asOf, history = undefined) => {
    return wasm.rebalanceComposite(
      json(instrument),
      json(market),
      asOf,
      history === undefined ? undefined : json(history)
    );
  },
  /** Price frozen primitive paths and return net/gross value and additive risk. */
  primitiveExposures: (instrument, market, asOf, metrics = undefined) => {
    return wasm.compositePrimitiveExposures(json(instrument), json(market), asOf, metrics);
  },
  /** Flatten target holdings or a state transition into primitive execution deltas. */
  executionTrades: (instrument, previous = undefined) => {
    return wasm.compositeExecutionTrades(
      json(instrument),
      previous === undefined ? undefined : json(previous)
    );
  },
  /** Initialize at the first snapshot and return dated P&L, return, index, exposure, and trade rows. */
  historyFromSpec: (spec, observations, warmup = undefined, metrics = undefined) => {
    return wasm.compositeHistoryFromSpec(
      json(spec),
      json(observations),
      warmup === undefined ? undefined : json(warmup),
      metrics
    );
  },
  /** Run dated history from an already-resolved immutable state. */
  history: (instrument, observations, metrics = undefined) => {
    return wasm.compositeHistory(json(instrument), json(observations), metrics);
  },
};
