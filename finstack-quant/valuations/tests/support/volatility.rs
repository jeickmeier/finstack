use finstack_quant_core::market_data::surfaces::VolSurface;

/// Build a constant vol surface using provided expiries/strikes grid.
pub fn flat_vol_surface(id: &str, expiries: &[f64], strikes: &[f64], vol: f64) -> VolSurface {
    let mut builder = VolSurface::builder(id).expiries(expiries).strikes(strikes);
    for _ in expiries {
        builder = builder.row(&vec![vol; strikes.len()]);
    }
    builder.build().expect("vol surface should build in tests")
}
