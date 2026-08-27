import * as wasm from '../../pkg/finstack_quant_wasm.js';

export const monteCarlo = {
  priceEuropeanCall: wasm.priceEuropeanCall,
  priceEuropeanPut: wasm.priceEuropeanPut,
  priceHestonCall: wasm.priceHestonCall,
  priceHestonPut: wasm.priceHestonPut,
  priceAsianCall: wasm.priceAsianCall,
  priceAsianPut: wasm.priceAsianPut,
  priceAmericanPut: wasm.priceAmericanPut,
  priceAmericanCall: wasm.priceAmericanCall,
  priceAmericanPutUnbiased: wasm.priceAmericanPutUnbiased,
  priceAmericanCallUnbiased: wasm.priceAmericanCallUnbiased,
  blackScholesCall: wasm.blackScholesCall,
  blackScholesPut: wasm.blackScholesPut,
};
