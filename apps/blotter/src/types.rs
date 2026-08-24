//! Book and row types for the blotter API and UI.
use serde::{Deserialize, Serialize};

/// Top-level book payload persisted and served by the blotter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    /// "paper" or "live" — UI shows PAPER by default
    pub mode: String,
    /// Whether live trading is enabled by the desk
    pub live_enabled: bool,
    /// As-of timestamp in UTC (ISO 8601 / RFC 3339)
    pub as_of: String,
    /// Freeform note from the desk
    #[serde(default)]
    pub notes: Option<String>,
    /// Risk limits
    pub risk: Risk,
    /// Kill switch status
    pub kill_switch: KillSwitch,
    /// Open quotes (CLOB) — current book
    #[serde(default)]
    pub quotes: Vec<Quote>,
    /// Inventory rows
    #[serde(default)]
    pub inventory: Vec<Inventory>,
    /// Fills, newest last in the vector (UI shows newest first)
    #[serde(default)]
    pub fills: Vec<Fill>,
    /// Realized and unrealized PnL, in USD
    pub pnl: Pnl,
    /// Last pricer sheet update time (UTC) if present
    #[serde(default)]
    pub last_pricer_sheet: Option<String>,
}

/// Risk/cap configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    /// Maximum absolute inventory (shares) allowed per token
    pub max_inventory_shares_per_token: i64,
    /// Maximum quote size (shares) the desk will post
    pub max_quote_size: i64,
    /// Maximum number of concurrently open markets
    pub max_open_markets: i64,
    /// Maximum absolute notional USD exposure
    pub max_notional_usd: i64,
    /// Book considered stale if older than this many seconds
    pub stale_after_seconds: i64,
    #[serde(default)]
    /// Conditions that trip the kill-switch automatically
    pub kill_on: Vec<String>,
}

/// Kill-switch state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitch {
    /// Whether the kill-switch is armed
    pub armed: bool,
    /// Whether the kill-switch has tripped
    pub tripped: bool,
    #[serde(default)]
    /// Reason provided when tripped
    pub reason: Option<String>,
    #[serde(default)]
    /// Timestamp the kill-switch tripped (UTC)
    pub tripped_at: Option<String>,
}

/// Open quote row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// Market identifier (e.g., "2026-07 Rain NYC")
    pub market: String,
    /// Token label (YES/NO or bin)
    pub token: String,
    /// Best bid
    pub bid: f64,
    /// Best ask
    pub ask: f64,
    /// Bid size (shares)
    pub bid_size: f64,
    /// Ask size (shares)
    pub ask_size: f64,
    #[serde(default)]
    /// Fair value estimate, if present
    pub fv: Option<f64>,
    /// Last update time (UTC)
    pub updated_at: String,
}

/// Inventory row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Market identifier
    pub market: String,
    /// City or locality
    pub city: String,
    /// Token label (YES/NO or bin)
    pub token: String,
    /// Net position in shares
    pub net_shares: f64,
    /// Average price of the position
    pub avg_price: f64,
    /// Current mark
    pub mark: f64,
    /// Unrealized PnL in USD
    pub unrealized_usd: f64,
    /// Absolute notional exposure in USD
    pub notional_usd: f64,
}

/// Fill row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    /// Unique fill identifier
    pub id: String,
    /// Fill timestamp in UTC (ISO 8601 / RFC 3339)
    pub ts: String,
    /// Market identifier
    pub market: String,
    /// City or locality
    pub city: String,
    /// Token label (YES/NO or bin)
    pub token: String,
    /// Side: buy or sell
    pub side: String,
    /// Executed price
    pub price: f64,
    /// Executed size (shares)
    pub size: f64,
    /// Notional in USD (price × size)
    pub notional_usd: f64,
    /// Fees in USD
    pub fee_usd: f64,
    /// Liquidity role: maker or taker
    pub liquidity: String,
    #[serde(default)]
    /// Mid price at time of fill, when known
    pub mid_at_fill: Option<f64>,
}

/// PnL totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pnl {
    /// Realized PnL in USD
    pub realized_usd: f64,
    /// Unrealized PnL in USD
    pub unrealized_usd: f64,
}

impl Default for Book {
    fn default() -> Self {
        // Exact flat book provided in the task description.
        serde_json::from_str::<Book>(
            r#"{
  "mode": "paper",
  "live_enabled": false,
  "as_of": "2026-08-24T22:15:43Z",
  "notes": "Paper book only. Live CLOB posts require Jon's explicit go-ahead.",
  "risk": {
    "max_inventory_shares_per_token": 50,
    "max_quote_size": 10,
    "max_open_markets": 3,
    "max_notional_usd": 200,
    "stale_after_seconds": 900,
    "kill_on": ["surprise_warning", "stale_price", "inventory_limit", "lost_data"]
  },
  "kill_switch": { "armed": true, "tripped": false, "reason": null, "tripped_at": null },
  "quotes": [],
  "inventory": [],
  "fills": [],
  "pnl": { "realized_usd": 0.0, "unrealized_usd": 0.0 },
  "last_pricer_sheet": null
}"#,
        )
        .expect("default book JSON is valid")
    }
}
