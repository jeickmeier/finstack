import * as wasm from '../pkg/finstack_quant_wasm.js';
import { correlation } from './models/correlation.js';
import { credit } from './models/credit.js';
import { monteCarlo } from './models/monteCarlo.js';

export const models = {
  correlation,
  credit,
  monteCarlo,
  SabrParameters: wasm.SabrParameters,
  SabrModel: wasm.SabrModel,
  SabrSmile: wasm.SabrSmile,
  SabrCalibrator: wasm.SabrCalibrator,
  bsPrice: wasm.bsPrice,
  vanillaExpiryPayoff: wasm.vanillaExpiryPayoff,
  bsGreeks: wasm.bsGreeks,
  bsImpliedVol: wasm.bsImpliedVol,
  black76ImpliedVol: wasm.black76ImpliedVol,
  barrierCall: wasm.barrierCall,
  asianOptionPrice: wasm.asianOptionPrice,
  lookbackOptionPrice: wasm.lookbackOptionPrice,
  quantoOptionPrice: wasm.quantoOptionPrice,
  bsCosPrice: wasm.bsCosPrice,
  vgCosPrice: wasm.vgCosPrice,
  mertonJumpCosPrice: wasm.mertonJumpCosPrice,
};
