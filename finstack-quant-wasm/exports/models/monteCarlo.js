import * as wasm from '../../pkg/finstack_quant_wasm.js';

export const monteCarlo = {
  priceHestonCall: wasm.priceHestonCall,
  priceHestonPut: wasm.priceHestonPut,
};
