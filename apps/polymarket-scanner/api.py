"""
Polymarket Scanner API
======================
Serves scanner SQLite data as JSON for the dashboard.

Run:
    python api.py              # port 4000
    python api.py --port 4001
"""
from __future__ import annotations

import json
import sqlite3
from pathlib import Path
from typing import Any

import uvicorn
from fastapi import FastAPI, Query
from fastapi.middleware.cors import CORSMiddleware

DB_PATH = Path(__file__).parent / "data" / "polymarket.db"

app = FastAPI(title="Polymarket Scanner API", version="0.2")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["GET"],
    allow_headers=["*"],
)


def _con() -> sqlite3.Connection:
    con = sqlite3.connect(DB_PATH)
    con.row_factory = sqlite3.Row
    return con


def _row_to_dict(row: sqlite3.Row) -> dict[str, Any]:
    d = dict(row)
    for key in ("outcomes_json",):
        if key in d and d[key]:
            try:
                d[key] = json.loads(d[key])
            except Exception:
                pass
    return d


# ── /health ───────────────────────────────────────────────────────────────────

@app.get("/health")
def health():
    try:
        with _con() as con:
            count = con.execute("SELECT COUNT(*) FROM snapshots").fetchone()[0]
        return {"ok": True, "db_rows": count, "db": str(DB_PATH)}
    except Exception as e:
        return {"ok": False, "error": str(e)}


# ── /markets ──────────────────────────────────────────────────────────────────

@app.get("/markets")
def markets(
    limit: int = Query(50, le=200),
    min_volume: float = Query(0),
    sort: str = Query("edge_score"),
):
    """
    Latest snapshot of each market, ranked by edge_score (default).
    One row per condition_id from the most recent scan timestamp.
    """
    allowed_sorts = {
        "edge_score", "volume_24h", "volume_total", "liquidity",
        "arb_deviation", "competitive", "spread",
    }
    order_col = sort if sort in allowed_sorts else "edge_score"

    sql = f"""
        SELECT s.*
        FROM snapshots s
        INNER JOIN (
            SELECT condition_id, MAX(ts) AS max_ts
            FROM snapshots
            GROUP BY condition_id
        ) latest ON s.condition_id = latest.condition_id AND s.ts = latest.max_ts
        WHERE s.volume_24h >= ?
        ORDER BY s.{order_col} DESC NULLS LAST
        LIMIT ?
    """
    with _con() as con:
        rows = con.execute(sql, (min_volume, limit)).fetchall()
    return [_row_to_dict(r) for r in rows]


# ── /market/:cid ──────────────────────────────────────────────────────────────

@app.get("/market/{condition_id}")
def market_history(condition_id: str, limit: int = Query(200, le=1000)):
    """Full time-series for one market (all snapshots, oldest first)."""
    sql = """
        SELECT * FROM snapshots
        WHERE condition_id = ?
        ORDER BY ts ASC
        LIMIT ?
    """
    with _con() as con:
        rows = con.execute(sql, (condition_id, limit)).fetchall()
    return [_row_to_dict(r) for r in rows]


# ── /stats ────────────────────────────────────────────────────────────────────

@app.get("/stats")
def stats():
    """Aggregate stats across the latest scan."""
    sql_latest_ts = "SELECT MAX(ts) FROM snapshots"
    sql = """
        SELECT
            COUNT(*)                          AS market_count,
            SUM(volume_24h)                   AS total_volume_24h,
            AVG(edge_score)                   AS avg_edge_score,
            MAX(edge_score)                   AS max_edge_score,
            SUM(CASE WHEN ABS(arb_deviation) > 0.02 THEN 1 ELSE 0 END) AS arb_signals,
            COUNT(DISTINCT condition_id)      AS unique_markets,
            MAX(ts)                           AS last_scan_ts,
            MIN(ts)                           AS first_scan_ts,
            (SELECT COUNT(*) FROM snapshots)  AS total_rows
        FROM snapshots
        WHERE ts = (SELECT MAX(ts) FROM snapshots)
    """
    with _con() as con:
        row = con.execute(sql).fetchone()
    return dict(row) if row else {}


# ── /arb ─────────────────────────────────────────────────────────────────────

@app.get("/arb")
def arb_signals(min_deviation: float = Query(0.01)):
    """Markets where YES+NO mid deviates significantly from 1.0."""
    sql = """
        SELECT s.*
        FROM snapshots s
        INNER JOIN (
            SELECT condition_id, MAX(ts) AS max_ts
            FROM snapshots
            GROUP BY condition_id
        ) latest ON s.condition_id = latest.condition_id AND s.ts = latest.max_ts
        WHERE ABS(s.arb_deviation) >= ?
        ORDER BY ABS(s.arb_deviation) DESC
        LIMIT 50
    """
    with _con() as con:
        rows = con.execute(sql, (min_deviation,)).fetchall()
    return [_row_to_dict(r) for r in rows]


# ── /top ─────────────────────────────────────────────────────────────────────

@app.get("/top")
def top_opportunities(limit: int = Query(10, le=50)):
    """Highest edge_score markets from the latest scan."""
    sql = """
        SELECT s.condition_id, s.question, s.edge_score, s.volume_24h,
               s.liquidity, s.spread, s.arb_deviation, s.competitive,
               s.days_to_expiry, s.price_change_1wk, s.outcomes_json, s.ts
        FROM snapshots s
        INNER JOIN (
            SELECT condition_id, MAX(ts) AS max_ts
            FROM snapshots GROUP BY condition_id
        ) latest ON s.condition_id = latest.condition_id AND s.ts = latest.max_ts
        ORDER BY s.edge_score DESC
        LIMIT ?
    """
    with _con() as con:
        rows = con.execute(sql, (limit,)).fetchall()
    return [_row_to_dict(r) for r in rows]


# ── /flow ─────────────────────────────────────────────────────────────────────

@app.get("/flow")
def strong_flow(limit: int = Query(20, le=100)):
    """
    Markets with strongly one-sided order flow on the YES outcome.
    Returns markets where buy_ratio is far from 0.5.
    """
    sql = """
        SELECT s.condition_id, s.question, s.volume_24h, s.edge_score,
               s.outcomes_json, s.ts
        FROM snapshots s
        INNER JOIN (
            SELECT condition_id, MAX(ts) AS max_ts
            FROM snapshots GROUP BY condition_id
        ) latest ON s.condition_id = latest.condition_id AND s.ts = latest.max_ts
        WHERE s.outcomes_json IS NOT NULL
        ORDER BY s.edge_score DESC
        LIMIT ?
    """
    with _con() as con:
        rows = con.execute(sql, (limit,)).fetchall()

    result = []
    for row in rows:
        d = _row_to_dict(row)
        outcomes = d.get("outcomes_json") or []
        yes = outcomes[0] if outcomes else {}
        ratio = yes.get("buy_ratio")
        if ratio is not None and abs(ratio - 0.5) >= 0.2:
            d["yes_buy_ratio"] = ratio
            d["flow_skew"] = ratio - 0.5
            result.append(d)

    result.sort(key=lambda x: abs(x.get("flow_skew", 0)), reverse=True)
    return result[:limit]


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--port", type=int, default=4000)
    p.add_argument("--host", default="127.0.0.1")
    args = p.parse_args()
    print(f"Polymarket API starting on http://{args.host}:{args.port}")
    uvicorn.run(app, host=args.host, port=args.port, log_level="warning")
