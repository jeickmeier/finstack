//! Derived metrics helpers (pure functions) used by the UI and tests.

use crate::types::Book;
use chrono::{DateTime, Duration, Utc};

/// Return whether the book is stale as of `now`.
///
/// The book is stale if:
/// - `now - as_of > stale_after_seconds`, OR
/// - `last_pricer_sheet` exists and is older than `stale_after_seconds`
///
/// # Arguments
/// - `book`: The book payload.
/// - `now`: The current UTC time for evaluation.
pub fn is_stale(book: &Book, now: DateTime<Utc>) -> bool {
    let stale_after = Duration::seconds(book.risk.stale_after_seconds);

    // Parse helper that returns None on failure.
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    };

    let as_of_ok = parse(&book.as_of);
    if let Some(as_of) = as_of_ok {
        if now - as_of > stale_after {
            return true;
        }
    }

    if let Some(ref sheet_ts) = book.last_pricer_sheet {
        if let Some(sheet_time) = parse(sheet_ts) {
            if now - sheet_time > stale_after {
                return true;
            }
        }
    }

    false
}

/// Sum realized and unrealized PnL for a simple total.
///
/// # Arguments
/// - `book`: The book payload.
pub fn pnl_total(book: &Book) -> f64 {
    book.pnl.realized_usd + book.pnl.unrealized_usd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Book;

    #[test]
    fn test_stale_flag_math() {
        let mut b = Book::default();
        // stale_after_seconds = 900 (15 min)
        let now = chrono::DateTime::parse_from_rfc3339("2026-08-24T22:31:00Z")
            .unwrap()
            .with_timezone(&Utc); // 930s after as_of → stale
        assert!(is_stale(&b, now));

        // fresh: within 5 minutes
        let fresh_now = chrono::DateTime::parse_from_rfc3339("2026-08-24T22:19:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!is_stale(&b, fresh_now));

        // fresh as_of but pricer sheet too old
        b.as_of = "2026-08-24T22:19:00Z".to_string();
        b.last_pricer_sheet = Some("2026-08-24T22:00:00Z".to_string());
        let now2 = chrono::DateTime::parse_from_rfc3339("2026-08-24T22:20:01Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(is_stale(&b, now2)); // sheet older than 15 min
    }

    #[test]
    fn test_pnl_total() {
        let mut b = Book::default();
        b.pnl.realized_usd = 12.5;
        b.pnl.unrealized_usd = -2.25;
        assert!((pnl_total(&b) - 10.25).abs() < 1e-9);
    }
}
