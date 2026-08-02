use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::term_structures::PriceCurve;

/// Build a flat price curve with a constant price level (for commodity forward prices).
pub fn flat_price_curve(id: &str, as_of: Date, price: f64, tenor_years: f64) -> PriceCurve {
    PriceCurve::builder(id)
        .base_date(as_of)
        .spot_price(price)
        .knots([(0.0, price), (tenor_years, price)])
        .build()
        .expect("price curve should build in tests")
}

/// Build a contango price curve (forward prices increase with time).
pub fn contango_price_curve(
    id: &str,
    as_of: Date,
    spot: f64,
    carry_rate: f64,
    tenor_years: f64,
) -> PriceCurve {
    let far_price = spot * (carry_rate * tenor_years).exp();
    PriceCurve::builder(id)
        .base_date(as_of)
        .spot_price(spot)
        .knots([(0.0, spot), (tenor_years, far_price)])
        .build()
        .expect("price curve should build in tests")
}
