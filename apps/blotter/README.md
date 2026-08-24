### Blotter (Polymarket weather desk)

Desktop-first, dense, dark dashboard showing the live or paper book. Not a trading or pricing tool — reporting only. No weather commentary, no seeded live fills.

What it shows
- As-of in America/Toronto and UTC, PAPER/LIVE badge, live_enabled, kill-switch armed/tripped, stale flag
- KPIs: fills today, inventory (net + notional), PnL realized/unrealized/total, hit rate (maker%), open quotes, markets vs cap, notional vs cap, spread capture vs directional PnL (only if `mid_at_fill` is present)
- Fills tape (newest first), Inventory, Open quotes, Limits & caps, Kill-switch, Flags

Live updates
- Server-Sent Events at `/api/stream` update the page without reload. Fallback polling every 5s if SSE is unavailable.

API
- GET `/api/book` → current book JSON
- POST `/api/book` → replace the current book
  - Authorization: `Authorization: Bearer $BOOK_INGEST_TOKEN`
  - Body: the full book JSON (see default below)
- GET `/api/demo` → demo/sample book (never mixed into live)
- Static dashboard at `/` (served from `apps/blotter/static/`)

Auth
- Set `BOOK_INGEST_TOKEN` in the blotter process environment. POST requests must include `Authorization: Bearer $BOOK_INGEST_TOKEN`. GET is open.

Persistence
- The server persists the last ingested book at `apps/blotter/data/book.json` (configurable via `BLOTTER_STORAGE_PATH`). On first run, it writes the exact flat default below.

Default book (exact, checked in)

```json
{
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
}
```

Run locally
1. Ensure Rust toolchain is installed via `mise install`
2. Set env: `export BOOK_INGEST_TOKEN=changeme`
3. Run the server:
   - `mise run rust-build-crate -- finstack-blotter`
   - `cargo run -p finstack-blotter` (or `BLOTTER_ADDR=0.0.0.0:8787 cargo run -p finstack-blotter`)
4. Open `http://localhost:8787/?demo=1` for demo mode (banner “DEMO DATA — not the desk”)
5. Open `http://localhost:8787/` for live mode (serves the persisted book)

Quote Desk — POST the paper book

```bash
curl -X POST http://localhost:8787/api/book \
  -H "authorization: Bearer $BOOK_INGEST_TOKEN" \
  -H "content-type: application/json" \
  --data-binary @/path/to/current_book.json
```

Config
- `BLOTTER_ADDR` (default `127.0.0.1:8787`)
- `BOOK_INGEST_TOKEN` (required for POST)
- `BLOTTER_STORAGE_PATH` (default `apps/blotter/data/book.json`)
- `BLOTTER_DEMO_PATH` (default `apps/blotter/fixtures/demo_book.json` if provided via env, else falls back to the flat default)

Tests
- Ingest auth, stale flag math, PnL totals, and the empty-book default are covered in `apps/blotter/tests/`.
