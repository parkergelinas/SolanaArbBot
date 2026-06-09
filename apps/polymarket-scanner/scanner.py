"""
Polymarket Advanced Scanner
===========================
Fetches live markets, computes edge signals, stores to SQLite, and
renders a ranked opportunity table in the terminal.

Usage
-----
    python scanner.py                        # continuous scan every 60 s
    python scanner.py --interval 30          # scan every 30 s
    python scanner.py --min-volume 5000      # higher volume gate
    python scanner.py --once                 # single scan then exit
    python scanner.py --pages 5              # limit Gamma API pages

Env
---
    POLY_API_KEY   - Polymarket CLOB API key (loaded from .env automatically)
    POLY_DB_PATH   - SQLite path (default: data/polymarket.db)
"""

from __future__ import annotations

import asyncio
import json
import math
import os
import re
import sqlite3
import sys
import time
from argparse import ArgumentParser
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import httpx
from dotenv import load_dotenv
from rich import box
from rich.console import Console
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

# ── Load .env ─────────────────────────────────────────────────────────────────
load_dotenv(Path(__file__).parent.parent.parent / ".env")
load_dotenv(Path(__file__).parent.parent.parent / ".env.local", override=True)

API_KEY = os.getenv("POLY_API_KEY", "")
DB_PATH = Path(os.getenv("POLY_DB_PATH", "data/polymarket.db"))

GAMMA_BASE = "https://gamma-api.polymarket.com"
CLOB_BASE  = "https://clob.polymarket.com"
DATA_BASE  = "https://data-api.polymarket.com"

CLOB_HEADERS = {"POLY_API_KEY": API_KEY} if API_KEY else {}

import io as _io
import sys as _sys
if _sys.platform == "win32":
    _sys.stdout = _io.TextIOWrapper(_sys.stdout.buffer, encoding="utf-8", errors="replace")
console = Console(highlight=False)


# ══════════════════════════════════════════════════════════════════════════════
# Data types
# ══════════════════════════════════════════════════════════════════════════════

@dataclass
class BookSide:
    levels: list[tuple[float, float]]   # [(price, size), ...]

    @classmethod
    def from_raw(cls, raw: list[dict]) -> "BookSide":
        levels = []
        for l in raw[:20]:
            try:
                levels.append((float(l["price"]), float(l["size"])))
            except (KeyError, ValueError):
                pass
        return cls(levels)

    @property
    def best(self) -> float | None:
        return self.levels[0][0] if self.levels else None

    def depth_within(self, cents: float) -> float:
        if not self.levels:
            return 0.0
        best = self.levels[0][0]
        thresh = cents / 100.0
        return sum(s for p, s in self.levels if abs(p - best) <= thresh)

    def total_depth(self) -> float:
        return sum(s for _, s in self.levels)

    def vwap(self, n: int = 5) -> float | None:
        top = self.levels[:n]
        vol = sum(s for _, s in top)
        if vol == 0:
            return None
        return sum(p * s for p, s in top) / vol


@dataclass
class OutcomeData:
    token_id: str
    name: str
    price: float | None
    bid: BookSide
    ask: BookSide
    last_trade_price: float | None
    last_trade_side: str | None
    buy_vol: float
    sell_vol: float
    trade_count: int
    trade_vwap: float | None
    last_trade_ts: int | None

    # Computed
    spread: float | None = field(default=None, init=False)
    mid: float | None = field(default=None, init=False)
    spread_pct: float | None = field(default=None, init=False)
    book_imbalance_10c: float | None = field(default=None, init=False)
    weighted_mid: float | None = field(default=None, init=False)

    def __post_init__(self):
        b, a = self.bid.best, self.ask.best
        if b is not None and a is not None:
            self.spread = a - b
            self.mid = (a + b) / 2
            self.spread_pct = self.spread / self.mid if self.mid > 0 else None
        elif b is not None:
            self.mid = b
        elif a is not None:
            self.mid = a

        bd = self.bid.depth_within(10)
        ad = self.ask.depth_within(10)
        total = bd + ad
        self.book_imbalance_10c = bd / total if total > 0 else None

        bv = self.bid.vwap(5)
        av = self.ask.vwap(5)
        if bv is not None and av is not None:
            bsize = sum(s for _, s in self.bid.levels[:5])
            asize = sum(s for _, s in self.ask.levels[:5])
            total_sz = bsize + asize
            if total_sz > 0:
                self.weighted_mid = (bv * bsize + av * asize) / total_sz

    @property
    def buy_ratio(self) -> float | None:
        total = self.buy_vol + self.sell_vol
        return self.buy_vol / total if total > 0 else None

    @property
    def prob(self) -> float | None:
        return self.mid or self.price


@dataclass
class MarketData:
    ts: datetime
    condition_id: str
    question: str
    slug: str | None
    event_title: str | None
    end_date: str | None
    days_to_expiry: float | None
    volume_24h: float | None
    volume_1wk: float | None
    volume_total: float | None
    liquidity: float | None
    spread: float | None
    competitive: float | None
    maker_fee_bps: int | None
    taker_fee_bps: int | None
    neg_risk: bool
    price_change_1wk: float | None
    price_change_1mo: float | None
    event_comment_count: int | None
    event_open_interest: float | None
    outcomes: list[OutcomeData]

    # Derived market-level signals
    price_sum: float | None = field(default=None, init=False)
    arb_deviation: float | None = field(default=None, init=False)
    vol_liq_ratio: float | None = field(default=None, init=False)
    edge_score: float = field(default=0.0, init=False)

    def __post_init__(self):
        mids = [o.mid for o in self.outcomes if o.mid is not None]
        if len(mids) >= 2:
            self.price_sum = sum(mids)
            self.arb_deviation = self.price_sum - 1.0

        if self.volume_24h and self.liquidity and self.liquidity > 0:
            self.vol_liq_ratio = self.volume_24h / self.liquidity

        self.edge_score = self._compute_edge_score()

    def _compute_edge_score(self) -> float:
        """
        Composite score (higher = more interesting for a trading bot).

        Components:
          • Uncertainty   – how close to 50/50 (max signal, not decided)
          • Momentum      – strong price_change_1wk in either direction
          • Liquidity     – log of volume_24h (deeper = easier execution)
          • Flow skew     – buy_ratio deviation from 0.5 (one-sided pressure)
          • Arb signal    – how far price_sum deviates from 1.0
          • Activity      – vol/liquidity ratio (active market)
        """
        score = 0.0

        # 1. Uncertainty: most interesting near 50/50
        for o in self.outcomes:
            if o.prob is not None:
                score += 1.0 - abs(o.prob - 0.5) * 2  # 1.0 at 50%, 0.0 at 0%/100%

        # 2. Momentum signal
        wk = self.price_change_1wk
        if wk is not None:
            score += min(abs(wk) * 10, 2.0)  # cap at 2

        # 3. Log-volume
        if self.volume_24h and self.volume_24h > 0:
            score += math.log10(self.volume_24h) / 5.0

        # 4. Order flow skew
        for o in self.outcomes:
            if o.buy_ratio is not None:
                score += abs(o.buy_ratio - 0.5) * 2

        # 5. Arb deviation (YES+NO ≠ 1.0)
        if self.arb_deviation is not None:
            score += abs(self.arb_deviation) * 5

        # 6. Activity
        if self.vol_liq_ratio is not None:
            score += min(self.vol_liq_ratio, 1.0)

        # 7. Social signal
        if self.event_comment_count and self.event_comment_count > 100:
            score += 0.5

        return round(score, 4)

    def kelly_fraction(self, outcome_idx: int, true_prob: float) -> float | None:
        """
        Kelly criterion bet size fraction given an estimated true probability.
        Uses the market mid as the available price (payoff = 1/price - 1).
        Returns None if mid is unavailable.
        """
        if outcome_idx >= len(self.outcomes):
            return None
        mid = self.outcomes[outcome_idx].mid
        if mid is None or mid <= 0 or mid >= 1:
            return None
        b = (1.0 / mid) - 1.0   # odds: net profit per unit staked
        q = 1.0 - true_prob
        kelly = (true_prob * b - q) / b
        return max(kelly, 0.0)


# ══════════════════════════════════════════════════════════════════════════════
# HTTP client
# ══════════════════════════════════════════════════════════════════════════════

class PolyClient:
    def __init__(self, concurrency: int = 8):
        limits = httpx.Limits(max_connections=concurrency, max_keepalive_connections=concurrency)
        self._http = httpx.AsyncClient(
            timeout=httpx.Timeout(connect=8.0, read=30.0, write=10.0, pool=5.0),
            limits=limits,
            headers={"User-Agent": "polymarket-scanner-py/0.2"},
        )
        self._sem = asyncio.Semaphore(concurrency)

    async def close(self):
        await self._http.aclose()

    async def _get(self, url: str, params: dict | None = None, auth: bool = False) -> Any:
        headers = CLOB_HEADERS if auth else {}
        async with self._sem:
            r = await self._http.get(url, params=params, headers=headers)
            r.raise_for_status()
            return r.json()

    # ── Gamma ──────────────────────────────────────────────────────────────────

    async def fetch_gamma_markets(self, pages: int = 5, min_volume: float = 500) -> list[dict]:
        all_markets = []
        limit = 100
        for page in range(pages):
            try:
                data = await self._get(
                    f"{GAMMA_BASE}/markets",
                    params={
                        "active": "true", "closed": "false",
                        "enableOrderBook": "true",
                        "order": "volume24hr", "ascending": "false",
                        "limit": limit, "offset": page * limit,
                    },
                )
                if not isinstance(data, list) or not data:
                    break
                # Volume filter early
                data = [m for m in data if (m.get("volume24hr") or 0) >= min_volume]
                all_markets.extend(data)
                if len(data) < limit:
                    break
            except Exception as e:
                console.print(f"[red]Gamma page {page} failed: {e}[/]")
                break
        return all_markets

    # ── CLOB ───────────────────────────────────────────────────────────────────

    async def fetch_book(self, token_id: str) -> dict | None:
        try:
            return await self._get(f"{CLOB_BASE}/book", params={"token_id": token_id}, auth=True)
        except Exception:
            return None

    async def fetch_last_trade(self, token_id: str) -> dict | None:
        try:
            return await self._get(
                f"{CLOB_BASE}/last-trade-price", params={"token_id": token_id}, auth=True
            )
        except Exception:
            return None

    async def fetch_midpoint(self, token_id: str) -> float | None:
        try:
            r = await self._get(
                f"{CLOB_BASE}/midpoint", params={"token_id": token_id}, auth=True
            )
            return float(r.get("mid", 0) or 0) or None
        except Exception:
            return None

    # ── Data API ───────────────────────────────────────────────────────────────

    async def fetch_trades(self, condition_id: str, limit: int = 100) -> list[dict]:
        try:
            data = await self._get(
                f"{DATA_BASE}/trades",
                params={"market": condition_id, "limit": limit},
            )
            return data if isinstance(data, list) else []
        except Exception:
            return []


# ══════════════════════════════════════════════════════════════════════════════
# Scanner
# ══════════════════════════════════════════════════════════════════════════════

def _parse_stringified(val: Any) -> list | None:
    """Gamma returns lists as JSON strings sometimes."""
    if val is None:
        return None
    if isinstance(val, list):
        return val
    if isinstance(val, str):
        try:
            parsed = json.loads(val)
            return parsed if isinstance(parsed, list) else None
        except json.JSONDecodeError:
            return None
    return None


def _days_to_expiry(iso: str | None) -> float | None:
    if not iso:
        return None
    try:
        end = datetime.fromisoformat(iso.replace("Z", "+00:00"))
        return (end - datetime.now(timezone.utc)).total_seconds() / 86400
    except Exception:
        return None


async def scan_once(client: PolyClient, pages: int, min_volume: float) -> list[MarketData]:
    raw_markets = await client.fetch_gamma_markets(pages=pages, min_volume=min_volume)
    console.print(f"[dim]Fetched {len(raw_markets)} markets from Gamma[/]")

    results: list[MarketData] = []

    for raw in raw_markets:
        cid = raw.get("conditionId") or raw.get("condition_id", "")
        if not cid:
            continue

        token_ids = _parse_stringified(raw.get("clobTokenIds")) or []
        outcome_names = _parse_stringified(raw.get("outcomes")) or []
        outcome_prices_raw = _parse_stringified(raw.get("outcomePrices")) or []
        outcome_prices = [float(p) for p in outcome_prices_raw if p is not None]

        if not token_ids:
            continue

        # Fetch trades once per market
        trades_raw = await client.fetch_trades(cid, limit=100)

        # Fetch all token data concurrently
        async def fetch_token(i: int, tid: str) -> OutcomeData:
            book_raw, last_raw = await asyncio.gather(
                client.fetch_book(tid),
                client.fetch_last_trade(tid),
                return_exceptions=True,
            )

            book_raw = book_raw if isinstance(book_raw, dict) else {}
            last_raw = last_raw if isinstance(last_raw, dict) else {}

            bid = BookSide.from_raw(book_raw.get("bids", []))
            ask = BookSide.from_raw(book_raw.get("asks", []))

            ltp = last_raw.get("price")
            lts = last_raw.get("side")

            # Trade stats for this token
            token_trades = [t for t in trades_raw if t.get("asset") == tid]
            buy_vol = sum(t.get("size", 0) for t in token_trades if t.get("side") == "BUY")
            sell_vol = sum(t.get("size", 0) for t in token_trades if t.get("side") == "SELL")
            prices = [t["price"] for t in token_trades if t.get("price") is not None]
            sizes  = [t["size"]  for t in token_trades if t.get("size")  is not None]
            vwap = (
                sum(p * s for p, s in zip(prices, sizes)) / sum(sizes)
                if sizes else None
            )
            last_ts = (
                max(t["timestamp"] for t in token_trades if t.get("timestamp"))
                if token_trades else None
            )

            gamma_price = outcome_prices[i] if i < len(outcome_prices) else None

            return OutcomeData(
                token_id=tid,
                name=outcome_names[i] if i < len(outcome_names) else f"Outcome {i}",
                price=gamma_price,
                bid=bid,
                ask=ask,
                last_trade_price=float(ltp) if ltp else None,
                last_trade_side=lts,
                buy_vol=buy_vol,
                sell_vol=sell_vol,
                trade_count=len(token_trades),
                trade_vwap=vwap,
                last_trade_ts=last_ts,
            )

        outcome_tasks = [fetch_token(i, tid) for i, tid in enumerate(token_ids)]
        outcome_list = list(await asyncio.gather(*outcome_tasks))

        # Event context
        events = raw.get("events") or []
        event = events[0] if events else {}

        end_date = raw.get("endDateIso") or raw.get("endDate")
        dte = _days_to_expiry(end_date)

        md = MarketData(
            ts=datetime.now(timezone.utc),
            condition_id=cid,
            question=raw.get("question", ""),
            slug=raw.get("slug"),
            event_title=event.get("title"),
            end_date=end_date,
            days_to_expiry=dte,
            volume_24h=raw.get("volume24hr"),
            volume_1wk=raw.get("volume1wk"),
            volume_total=raw.get("volumeNum"),
            liquidity=raw.get("liquidityNum") or raw.get("liquidityClob"),
            spread=raw.get("spread"),
            competitive=raw.get("competitive"),
            maker_fee_bps=raw.get("makerBaseFee"),
            taker_fee_bps=raw.get("takerBaseFee"),
            neg_risk=bool(raw.get("negRisk")),
            price_change_1wk=raw.get("oneWeekPriceChange"),
            price_change_1mo=raw.get("oneMonthPriceChange"),
            event_comment_count=event.get("commentCount"),
            event_open_interest=event.get("openInterest"),
            outcomes=outcome_list,
        )
        results.append(md)

    results.sort(key=lambda m: m.edge_score, reverse=True)
    return results


# ══════════════════════════════════════════════════════════════════════════════
# SQLite storage
# ══════════════════════════════════════════════════════════════════════════════

def init_db(path: Path) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(path)
    con.execute("""
        CREATE TABLE IF NOT EXISTS snapshots (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            ts              TEXT    NOT NULL,
            condition_id    TEXT    NOT NULL,
            question        TEXT,
            slug            TEXT,
            event_title     TEXT,
            end_date        TEXT,
            days_to_expiry  REAL,
            volume_24h      REAL,
            volume_1wk      REAL,
            volume_total    REAL,
            liquidity       REAL,
            vol_liq_ratio   REAL,
            spread          REAL,
            competitive     REAL,
            maker_fee_bps   INTEGER,
            taker_fee_bps   INTEGER,
            neg_risk        INTEGER,
            price_change_1wk REAL,
            price_change_1mo REAL,
            event_comments  INTEGER,
            event_oi        REAL,
            price_sum       REAL,
            arb_deviation   REAL,
            edge_score      REAL,
            outcomes_json   TEXT
        )
    """)
    con.execute("""
        CREATE INDEX IF NOT EXISTS idx_snap_cid ON snapshots(condition_id)
    """)
    con.execute("""
        CREATE INDEX IF NOT EXISTS idx_snap_ts ON snapshots(ts)
    """)
    con.commit()
    return con


def _outcome_to_dict(o: OutcomeData) -> dict:
    return {
        "token_id": o.token_id,
        "name": o.name,
        "price": o.price,
        "bid_best": o.bid.best,
        "ask_best": o.ask.best,
        "mid": o.mid,
        "spread": o.spread,
        "spread_pct": o.spread_pct,
        "last_trade_price": o.last_trade_price,
        "last_trade_side": o.last_trade_side,
        "buy_vol": o.buy_vol,
        "sell_vol": o.sell_vol,
        "buy_ratio": o.buy_ratio,
        "trade_count": o.trade_count,
        "trade_vwap": o.trade_vwap,
        "book_imbalance_10c": o.book_imbalance_10c,
        "weighted_mid": o.weighted_mid,
        "depth_1c_bid": o.bid.depth_within(1),
        "depth_5c_bid": o.bid.depth_within(5),
        "depth_10c_bid": o.bid.depth_within(10),
        "depth_1c_ask": o.ask.depth_within(1),
        "depth_5c_ask": o.ask.depth_within(5),
        "depth_10c_ask": o.ask.depth_within(10),
        "total_bid_depth": o.bid.total_depth(),
        "total_ask_depth": o.ask.total_depth(),
        "bid_vwap": o.bid.vwap(),
        "ask_vwap": o.ask.vwap(),
        "prob": o.prob,
        "prob_dist_50": abs(o.prob - 0.5) if o.prob is not None else None,
    }


def store_snapshots(con: sqlite3.Connection, markets: list[MarketData]):
    rows = []
    for m in markets:
        outcomes_json = json.dumps([_outcome_to_dict(o) for o in m.outcomes])
        rows.append((
            m.ts.isoformat(),
            m.condition_id,
            m.question,
            m.slug,
            m.event_title,
            m.end_date,
            m.days_to_expiry,
            m.volume_24h,
            m.volume_1wk,
            m.volume_total,
            m.liquidity,
            m.vol_liq_ratio,
            m.spread,
            m.competitive,
            m.maker_fee_bps,
            m.taker_fee_bps,
            int(m.neg_risk),
            m.price_change_1wk,
            m.price_change_1mo,
            m.event_comment_count,
            m.event_open_interest,
            m.price_sum,
            m.arb_deviation,
            m.edge_score,
            outcomes_json,
        ))
    con.executemany("""
        INSERT INTO snapshots (
            ts, condition_id, question, slug, event_title, end_date,
            days_to_expiry, volume_24h, volume_1wk, volume_total, liquidity,
            vol_liq_ratio, spread, competitive, maker_fee_bps, taker_fee_bps,
            neg_risk, price_change_1wk, price_change_1mo, event_comments,
            event_oi, price_sum, arb_deviation, edge_score, outcomes_json
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
    """, rows)
    con.commit()


# ══════════════════════════════════════════════════════════════════════════════
# Terminal UI
# ══════════════════════════════════════════════════════════════════════════════

def _fmt_pct(v: float | None, color: bool = True) -> str:
    if v is None:
        return "[dim]--[/]"
    pct = v * 100
    s = f"{pct:+.1f}%"
    if not color:
        return s
    if pct > 2:
        return f"[green]{s}[/]"
    if pct < -2:
        return f"[red]{s}[/]"
    return f"[yellow]{s}[/]"


def _fmt_prob(v: float | None) -> str:
    if v is None:
        return "[dim]--[/]"
    pct = v * 100
    if pct >= 75:
        return f"[green]{pct:.1f}%[/]"
    if pct <= 25:
        return f"[red]{pct:.1f}%[/]"
    return f"[yellow]{pct:.1f}%[/]"


def _fmt_arb(dev: float | None) -> str:
    if dev is None:
        return "[dim]--[/]"
    pct = dev * 100
    if abs(pct) < 1:
        return f"[dim]{pct:+.2f}%[/]"
    if pct > 0:
        return f"[bold green]{pct:+.2f}%[/]"
    return f"[bold red]{pct:+.2f}%[/]"


def _fmt_imbalance(v: float | None) -> str:
    if v is None:
        return "[dim]--[/]"
    if v > 0.65:
        return f"[green]{v:.2f}[/]"   # strong bid pressure
    if v < 0.35:
        return f"[red]{v:.2f}[/]"     # strong ask pressure
    return f"{v:.2f}"


def _fmt_vol(v: float | None) -> str:
    if v is None:
        return "[dim]--[/]"
    if v >= 1_000_000:
        return f"${v/1_000_000:.1f}M"
    if v >= 1_000:
        return f"${v/1_000:.1f}K"
    return f"${v:.0f}"


def _fmt_dte(v: float | None) -> str:
    if v is None:
        return "[dim]--[/]"
    if v < 0:
        return "[red]expired[/]"
    if v < 1:
        return f"[red]{v*24:.1f}h[/]"
    if v < 3:
        return f"[yellow]{v:.1f}d[/]"
    return f"{v:.1f}d"


def build_table(markets: list[MarketData], top_n: int = 25) -> Table:
    t = Table(
        title="Polymarket — Top Opportunities",
        box=box.SIMPLE_HEAVY,
        show_header=True,
        header_style="bold cyan",
        expand=True,
    )
    t.add_column("Score",    style="bold", width=6, justify="right")
    t.add_column("Question", min_width=30, ratio=4)
    t.add_column("Vol 24h",  width=9,  justify="right")
    t.add_column("Liq",      width=8,  justify="right")
    t.add_column("DTE",      width=7,  justify="right")
    t.add_column("Yes%",     width=7,  justify="right")
    t.add_column("No%",      width=7,  justify="right")
    t.add_column("Spread",   width=7,  justify="right")
    t.add_column("Imbal",    width=7,  justify="right")
    t.add_column("Flow",     width=7,  justify="right")  # buy ratio YES
    t.add_column("Arb",      width=8,  justify="right")
    t.add_column("1wk chg",  width=8,  justify="right")

    for m in markets[:top_n]:
        yes = m.outcomes[0] if m.outcomes else None
        no  = m.outcomes[1] if len(m.outcomes) > 1 else None

        yes_prob = _fmt_prob(yes.prob if yes else None)
        no_prob  = _fmt_prob(no.prob  if no  else None)

        spread_pct = yes.spread_pct if yes else m.spread
        spread_str = f"{spread_pct*100:.1f}%" if spread_pct else "[dim]--[/]"

        imbal = _fmt_imbalance(yes.book_imbalance_10c if yes else None)
        flow  = _fmt_imbalance(yes.buy_ratio if yes else None)

        q = m.question
        if len(q) > 60:
            q = q[:57] + "…"

        t.add_row(
            f"{m.edge_score:.2f}",
            q,
            _fmt_vol(m.volume_24h),
            _fmt_vol(m.liquidity),
            _fmt_dte(m.days_to_expiry),
            yes_prob,
            no_prob,
            spread_str,
            imbal,
            flow,
            _fmt_arb(m.arb_deviation),
            _fmt_pct(m.price_change_1wk),
        )

    return t


def build_stats_panel(markets: list[MarketData], db_path: Path) -> Panel:
    n = len(markets)
    arb_count = sum(1 for m in markets if m.arb_deviation and abs(m.arb_deviation) > 0.02)
    avg_score = sum(m.edge_score for m in markets) / n if n else 0
    total_vol = sum(m.volume_24h or 0 for m in markets)
    db_rows = 0
    try:
        with sqlite3.connect(db_path) as con:
            db_rows = con.execute("SELECT COUNT(*) FROM snapshots").fetchone()[0]
    except Exception:
        pass

    now = datetime.now(timezone.utc).strftime("%H:%M:%S UTC")
    text = (
        f"[bold]{now}[/]  "
        f"Markets: [cyan]{n}[/]  "
        f"Avg score: [cyan]{avg_score:.2f}[/]  "
        f"Total 24h vol: [cyan]{_fmt_vol(total_vol)}[/]  "
        f"Arb signals: [{'green' if arb_count else 'dim'}]{arb_count}[/]  "
        f"DB rows: [dim]{db_rows:,}[/]"
    )
    return Panel(text, title="Scanner Stats", border_style="dim")


def build_arb_table(markets: list[MarketData]) -> Table | None:
    arb = [m for m in markets if m.arb_deviation and abs(m.arb_deviation) > 0.01]
    if not arb:
        return None
    t = Table(title="[!] Arbitrage Signals (|YES+NO - 1| > 1%)", box=box.MINIMAL, header_style="bold yellow")
    t.add_column("Question", ratio=3)
    t.add_column("YES",  width=7, justify="right")
    t.add_column("NO",   width=7, justify="right")
    t.add_column("Sum",  width=7, justify="right")
    t.add_column("Dev",  width=8, justify="right")
    t.add_column("Vol",  width=9, justify="right")
    for m in sorted(arb, key=lambda x: abs(x.arb_deviation or 0), reverse=True)[:10]:
        yes = m.outcomes[0].mid if m.outcomes else None
        no  = m.outcomes[1].mid if len(m.outcomes) > 1 else None
        t.add_row(
            m.question[:55],
            f"{yes*100:.1f}%" if yes else "--",
            f"{no*100:.1f}%"  if no  else "--",
            f"{(m.price_sum or 0)*100:.2f}%",
            _fmt_arb(m.arb_deviation),
            _fmt_vol(m.volume_24h),
        )
    return t


# ══════════════════════════════════════════════════════════════════════════════
# Main loop
# ══════════════════════════════════════════════════════════════════════════════

async def run(interval: int, min_volume: float, pages: int, once: bool):
    con = init_db(DB_PATH)
    client = PolyClient(concurrency=8)

    try:
        while True:
            with console.status("[cyan]Scanning Polymarket…[/]", spinner="dots"):
                t0 = time.monotonic()
                markets = await scan_once(client, pages=pages, min_volume=min_volume)
                elapsed = time.monotonic() - t0

            store_snapshots(con, markets)

            console.clear()
            console.print(build_stats_panel(markets, DB_PATH))
            console.print(build_table(markets, top_n=30))

            arb_table = build_arb_table(markets)
            if arb_table:
                console.print(arb_table)

            console.print(
                f"\n[dim]Scan completed in {elapsed:.1f}s | "
                f"{len(markets)} markets stored | {DB_PATH}[/]"
            )

            if once:
                break

            console.print(f"[dim]Next scan in {interval}s… (Ctrl-C to quit)[/]")
            await asyncio.sleep(interval)

    except KeyboardInterrupt:
        console.print("\n[yellow]Stopped.[/]")
    finally:
        await client.close()
        con.close()


def main():
    ap = ArgumentParser(description="Polymarket Advanced Scanner")
    ap.add_argument("--interval",   type=int,   default=60,    help="Seconds between scans")
    ap.add_argument("--min-volume", type=float, default=500.0, help="Min 24h volume USD")
    ap.add_argument("--pages",      type=int,   default=5,     help="Gamma API pages (100 markets each)")
    ap.add_argument("--once",       action="store_true",       help="Run one scan then exit")
    args = ap.parse_args()

    if not API_KEY:
        console.print("[yellow]Warning: POLY_API_KEY not set — running unauthenticated[/]")

    asyncio.run(run(
        interval=args.interval,
        min_volume=args.min_volume,
        pages=args.pages,
        once=args.once,
    ))


if __name__ == "__main__":
    main()
