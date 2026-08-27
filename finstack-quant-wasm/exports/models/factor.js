import * as wasm from '../../pkg/finstack_quant_wasm.js';

const credit = {
  CreditFactorModel: wasm.CreditFactorModel,
  CreditCalibrator: wasm.CreditCalibrator,
  LevelsAtDate: wasm.LevelsAtDate,
  PeriodDecomposition: wasm.PeriodDecomposition,
  FactorCovarianceForecast: wasm.FactorCovarianceForecast,
  decomposeLevels: wasm.decomposeLevels,
  decomposePeriod: wasm.decomposePeriod,
};

const risk = {
  parametricVarDecomposition: wasm.parametricVarDecomposition,
  parametricEsDecomposition: wasm.parametricEsDecomposition,
  historicalVarDecomposition: wasm.historicalVarDecomposition,
  evaluateRiskBudget: wasm.evaluateRiskBudget,
};

export const factor = {
  credit,
  risk,
};
