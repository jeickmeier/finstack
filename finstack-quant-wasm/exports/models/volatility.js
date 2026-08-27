import * as wasm from '../../pkg/finstack_quant_wasm.js';

export const volatility = {
  SabrCalibrator: wasm.SabrCalibrator,
  SabrModel: wasm.SabrModel,
  SabrParameters: wasm.SabrParameters,
  SabrSmile: wasm.SabrSmile,
  deltaToStrike: wasm.deltaToStrike,
  getCubeNormalVol: wasm.getCubeNormalVol,
  getCubeNormalVolClamped: wasm.getCubeNormalVolClamped,
  getCubeVol: wasm.getCubeVol,
  getCubeVolClamped: wasm.getCubeVolClamped,
  getFxDeltaPillarVols: wasm.getFxDeltaPillarVols,
  getFxDeltaVol: wasm.getFxDeltaVol,
  strikeToDelta: wasm.strikeToDelta,
};
