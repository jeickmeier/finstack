const fmtUSD = (n) =>
  (n < 0 ? "-$" : "$") +
  Math.abs(n).toLocaleString(undefined, { maximumFractionDigits: 2, minimumFractionDigits: 2 });

const by = (k) => (a, b) => (a[k] < b[k] ? -1 : a[k] > b[k] ? 1 : 0);

function parseRfc3339(s) {
  // Date can parse RFC 3339; keep as Date
  const d = new Date(s);
  return isNaN(d.getTime()) ? null : d;
}

function toToronto(dtOrStr) {
  const d = typeof dtOrStr === "string" ? parseRfc3339(dtOrStr) : dtOrStr;
  if (!d) return "—";
  return new Intl.DateTimeFormat(undefined, {
    timeZone: "America/Toronto",
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(d);
}
function toUtc(dtOrStr) {
  const d = typeof dtOrStr === "string" ? parseRfc3339(dtOrStr) : dtOrStr;
  if (!d) return "—";
  return new Intl.DateTimeFormat(undefined, { timeZone: "UTC", dateStyle: "medium", timeStyle: "medium" }).format(d);
}

function isStale(book, now = new Date()) {
  const asOf = parseRfc3339(book.as_of);
  const staleAfterMs = (book?.risk?.stale_after_seconds ?? 0) * 1000;
  if (asOf && now - asOf > staleAfterMs) return true;
  if (book.last_pricer_sheet) {
    const sheet = parseRfc3339(book.last_pricer_sheet);
    if (sheet && now - sheet > staleAfterMs) return true;
  }
  return false;
}

function computeKpis(book) {
  const now = new Date();
  const todayToronto = new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Toronto",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  })
    .format(now)
    .replaceAll("/", "-"); // 2026-08-24 style

  const fillsToday =
    (book.fills || []).filter((f) => {
      const d = parseRfc3339(f.ts);
      if (!d) return false;
      const key = new Intl.DateTimeFormat("en-CA", {
        timeZone: "America/Toronto",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      })
        .format(d)
        .replaceAll("/", "-");
      return key === todayToronto;
    }).length || 0;

  const netShares = (book.inventory || []).reduce((acc, r) => acc + (r.net_shares || 0), 0);
  const invNotional = (book.inventory || []).reduce((acc, r) => acc + (r.notional_usd || 0), 0);

  const realized = book?.pnl?.realized_usd || 0;
  const unrealized = book?.pnl?.unrealized_usd || 0;
  const total = realized + unrealized;

  const totalFills = (book.fills || []).length;
  const makerFills = (book.fills || []).filter((f) => (f.liquidity || "").toLowerCase() === "maker").length;
  const hitRate = totalFills > 0 ? (makerFills / totalFills) * 100 : null;

  const openQuotes = (book.quotes || []).length;
  const uniqueMarkets = new Set((book.quotes || []).map((q) => q.market)).size;
  const marketCap = book?.risk?.max_open_markets || 0;
  const notionalCap = book?.risk?.max_notional_usd || 0;

  // Spread capture across fills with mid_at_fill present
  let spreadCapture = null;
  const withMid = (book.fills || []).filter((f) => typeof f.mid_at_fill === "number");
  if (withMid.length > 0) {
    spreadCapture = withMid.reduce((acc, f) => {
      const side = (f.side || "").toLowerCase();
      const mid = f.mid_at_fill ?? 0;
      const price = f.price ?? 0;
      const size = f.size ?? 0;
      let capture = 0;
      if (side === "buy") capture = (mid - price) * size;
      else if (side === "sell") capture = (price - mid) * size;
      return acc + capture;
    }, 0);
  }
  const directional = spreadCapture == null ? null : total - spreadCapture;

  return {
    stale: isStale(book, now),
    fillsToday,
    netShares,
    invNotional,
    realized,
    unrealized,
    total,
    hitRate,
    openQuotes,
    uniqueMarkets,
    marketCap,
    notionalCap,
    spreadCapture,
    directional,
  };
}

function renderTables(book) {
  // Fills (newest first)
  const fills = [...(book.fills || [])].sort((a, b) => (a.ts < b.ts ? 1 : -1));
  const fillsTable = document.getElementById("fills-table");
  fillsTable.innerHTML = `
    <thead>
      <tr>
        <th>TS (Toronto)</th><th>UTC</th><th>Market</th><th>City</th><th>Token</th><th>Side</th>
        <th>Price</th><th>Size</th><th>Notional</th><th>Fee</th><th>Liquidity</th><th>Mid@Fill</th>
      </tr>
    </thead>
    <tbody>
      ${fills
        .map((f) => {
          const sideClass = (f.side || "").toLowerCase() === "buy" ? "good" : "bad";
          return `<tr>
            <td>${toToronto(f.ts)}</td>
            <td class="muted">${toUtc(f.ts)}</td>
            <td>${f.market || ""}</td>
            <td>${f.city || ""}</td>
            <td>${f.token || ""}</td>
            <td class="${sideClass}">${(f.side || "").toUpperCase()}</td>
            <td>${(f.price ?? 0).toFixed(2)}</td>
            <td>${(f.size ?? 0).toFixed(2)}</td>
            <td>${fmtUSD(f.notional_usd ?? 0)}</td>
            <td class="muted">${fmtUSD(f.fee_usd ?? 0)}</td>
            <td>${(f.liquidity || "").toUpperCase()}</td>
            <td>${f.mid_at_fill == null ? "n/a" : (f.mid_at_fill ?? 0).toFixed(2)}</td>
          </tr>`;
        })
        .join("")}
    </tbody>`;

  // Inventory
  const inv = [...(book.inventory || [])].sort((a, b) => a.market.localeCompare(b.market) || a.city.localeCompare(b.city));
  const invTable = document.getElementById("inventory-table");
  invTable.innerHTML = `
    <thead>
      <tr>
        <th>Market</th><th>City</th><th>Token</th><th>Net Shares</th><th>Avg Px</th><th>Mark</th>
        <th>Unrealized</th><th>Notional</th>
      </tr>
    </thead>
    <tbody>
      ${inv
        .map(
          (r) => `<tr>
        <td>${r.market || ""}</td>
        <td>${r.city || ""}</td>
        <td>${r.token || ""}</td>
        <td>${(r.net_shares ?? 0).toFixed(2)}</td>
        <td>${(r.avg_price ?? 0).toFixed(2)}</td>
        <td>${(r.mark ?? 0).toFixed(2)}</td>
        <td class="${(r.unrealized_usd ?? 0) >= 0 ? "good" : "bad"}">${fmtUSD(r.unrealized_usd ?? 0)}</td>
        <td>${fmtUSD(r.notional_usd ?? 0)}</td>
      </tr>`
        )
        .join("")}
    </tbody>`;

  // Quotes
  const quotes = [...(book.quotes || [])].sort((a, b) => a.market.localeCompare(b.market) || a.token.localeCompare(b.token));
  const quotesTable = document.getElementById("quotes-table");
  quotesTable.innerHTML = `
    <thead>
      <tr>
        <th>Market</th><th>Token</th><th>Bid</th><th>Size</th><th>Ask</th><th>Size</th><th>FV</th><th>Updated</th>
      </tr>
    </thead>
    <tbody>
      ${quotes
        .map(
          (q) => `<tr>
        <td>${q.market || ""}</td>
        <td>${q.token || ""}</td>
        <td class="good">${(q.bid ?? 0).toFixed(2)}</td>
        <td>${(q.bid_size ?? 0).toFixed(2)}</td>
        <td class="bad">${(q.ask ?? 0).toFixed(2)}</td>
        <td>${(q.ask_size ?? 0).toFixed(2)}</td>
        <td class="muted">${q.fv == null ? "—" : (q.fv ?? 0).toFixed(2)}</td>
        <td class="muted">${toToronto(q.updated_at)}</td>
      </tr>`
        )
        .join("")}
    </tbody>`;

  // Limits & kill switch
  const risk = book.risk || {};
  document.getElementById("limits").innerHTML = `
    <div>Max inventory per token: <b>${risk.max_inventory_shares_per_token ?? "-"}</b></div>
    <div>Max quote size: <b>${risk.max_quote_size ?? "-"}</b></div>
    <div>Max open markets: <b>${risk.max_open_markets ?? "-"}</b></div>
    <div>Max notional USD: <b>${fmtUSD(risk.max_notional_usd ?? 0)}</b></div>
    <div>Stale after: <b>${risk.stale_after_seconds ?? 0}s</b></div>
    <div>Kill on: <b>${(risk.kill_on || []).join(", ")}</b></div>
  `;
  const ks = book.kill_switch || {};
  document.getElementById("killswitch").innerHTML = `
    <div>Armed: <b>${String(ks.armed)}</b></div>
    <div>Tripped: <b>${String(ks.tripped)}</b></div>
    <div>Reason: <b>${ks.reason || "—"}</b></div>
    <div>At: <b>${ks.tripped_at ? toToronto(ks.tripped_at) : "—"}</b></div>
  `;

  const flags = document.getElementById("flags");
  const fl = [];
  if (isStale(book)) fl.push("<li class='warn'>Stale book</li>");
  if (!book.risk) fl.push("<li class='warn'>Missing Risk</li>");
  if (!book.last_pricer_sheet) fl.push("<li class='warn'>Missing pricer sheet</li>");
  if (book.live_enabled && (book.mode || "paper").toLowerCase() !== "live")
    fl.push("<li class='warn'>live_enabled without live mode go-ahead</li>");
  flags.innerHTML = fl.length ? fl.join("") : "<li class='muted'>None</li>";
}

function renderHeader(book) {
  document.getElementById("mode-badge").textContent = (book.mode || "paper").toUpperCase();
  const live = !!book.live_enabled;
  const ks = book.kill_switch || {};
  const stale = isStale(book);
  const asof = parseRfc3339(book.as_of);
  document.getElementById("live-enabled").textContent = `live_enabled: ${live}`;
  document.getElementById("live-enabled").className = `pill ${live ? "pill-warn" : "pill-off"}`;
  document.getElementById("kill-armed").className = `pill ${ks.armed ? "pill-on" : "pill-off"}`;
  document.getElementById("kill-tripped").textContent = `kill tripped: ${!!ks.tripped}`;
  document.getElementById("kill-tripped").className = `pill ${ks.tripped ? "pill-err" : "pill-off"}`;
  document.getElementById("stale-flag").textContent = stale ? "stale" : "fresh";
  document.getElementById("stale-flag").className = `pill ${stale ? "pill-warn" : "pill-on"}`;
  document.getElementById("asof-toronto").textContent = `As of (Toronto): ${toToronto(asof)}`;
  document.getElementById("asof-utc").textContent = `UTC: ${toUtc(asof)}`;
}

function renderKpis(book) {
  const k = computeKpis(book);
  document.getElementById("fills-today").textContent = k.fillsToday;
  document.getElementById("inv-net").textContent = k.netShares.toFixed(2);
  document.getElementById("inv-notional").textContent = fmtUSD(k.invNotional);
  document.getElementById("pnl-realized").textContent = fmtUSD(k.realized);
  document.getElementById("pnl-unrealized").textContent = fmtUSD(k.unrealized);
  document.getElementById("pnl-total").textContent = fmtUSD(k.total);
  document.getElementById("hit-rate").textContent = k.hitRate == null ? "n/a" : `${k.hitRate.toFixed(0)}%`;
  document.getElementById("open-quotes").textContent = k.openQuotes;
  document.getElementById("markets-vs-cap").textContent = `${k.uniqueMarkets} / ${k.marketCap}`;
  document.getElementById("notional-vs-cap").textContent = `${fmtUSD(k.invNotional)} / ${fmtUSD(k.notionalCap)}`;
  document.getElementById("spread-capture").textContent = k.spreadCapture == null ? "n/a" : fmtUSD(k.spreadCapture);
  document.getElementById("directional-pnl").textContent = k.directional == null ? "n/a" : fmtUSD(k.directional);
}

function renderAll(book) {
  renderHeader(book);
  renderKpis(book);
  renderTables(book);
}

async function fetchJson(url) {
  const res = await fetch(url, { headers: { "cache-control": "no-cache" } });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return await res.json();
}

async function boot() {
  const url = new URL(window.location.href);
  const isDemo = url.searchParams.get("demo") === "1";
  if (isDemo) {
    document.getElementById("demo-banner").classList.remove("hidden");
  }

  const book = await fetchJson(isDemo ? "/api/demo" : "/api/book");
  renderAll(book);

  if (!isDemo) {
    try {
      const sse = new EventSource("/api/stream");
      sse.addEventListener("book", (ev) => {
        const next = JSON.parse(ev.data);
        renderAll(next);
      });
      sse.onerror = () => {
        // Will auto-reconnect; UI suggests refresh
      };
    } catch {
      // SSE not available; fall back to poll
      setInterval(async () => {
        try {
          const b = await fetchJson("/api/book");
          renderAll(b);
        } catch (_) {}
      }, 5000);
    }
  }
}

boot().catch((e) => {
  console.error(e);
  alert("Failed to load blotter.");
});
