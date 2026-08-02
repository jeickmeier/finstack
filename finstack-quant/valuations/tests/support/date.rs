use finstack_quant_core::dates::Date;
use time::Month;

/// Convenience date helper for tests.
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
        .expect("valid date")
}
