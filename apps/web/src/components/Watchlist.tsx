import { fmt } from "@/lib/formatters";
import { useState } from "react";

interface Token {
  symbol: string;
  price: number;
  change: number;
  volume: number;
}
interface Props {
  tokens: Token[];
}

export default function Watchlist({ tokens }: Props) {
  const [sortCol, setSortCol] = useState<"change" | "volume" | null>(null);
  const [sortAsc, setSortAsc] = useState(false);
  const [active, setActive] = useState(0);

  const sorted = [...tokens].sort((a, b) => {
    if (!sortCol) return 0;
    return sortAsc ? a[sortCol] - b[sortCol] : b[sortCol] - a[sortCol];
  });

  const toggleSort = (col: "change" | "volume") => {
    if (sortCol === col) setSortAsc((p) => !p);
    else {
      setSortCol(col);
      setSortAsc(false);
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "hidden",
      }}
    >
      <div className="panel-header">
        <span>WATCHLIST</span>
        <span
          style={{
            color: "var(--blue)",
            cursor: "pointer",
            fontSize: 14,
            fontWeight: 400,
          }}
        >
          +
        </span>
      </div>
      {/* Headers */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 80px 52px 52px",
          padding: "4px 12px",
          fontSize: 10,
          color: "var(--text-muted)",
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          flexShrink: 0,
        }}
      >
        <span>TOKEN</span>
        <span style={{ textAlign: "right" }}>PRICE</span>
        <span
          style={{ textAlign: "right", cursor: "pointer" }}
          onClick={() => toggleSort("change")}
        >
          CHG {sortCol === "change" ? (sortAsc ? "▲" : "▼") : ""}
        </span>
        <span
          style={{ textAlign: "right", cursor: "pointer" }}
          onClick={() => toggleSort("volume")}
        >
          VOL {sortCol === "volume" ? (sortAsc ? "▲" : "▼") : ""}
        </span>
      </div>
      {/* Rows */}
      <div style={{ overflow: "auto", flex: 1 }}>
        {sorted.map((token, i) => (
          <div
            key={token.symbol}
            onClick={() => setActive(i)}
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 80px 52px 52px",
              height: 28,
              alignItems: "center",
              padding: "0 12px",
              cursor: "pointer",
              borderLeft:
                active === i
                  ? "2px solid var(--blue)"
                  : "2px solid transparent",
              background: active === i ? "var(--bg-elevated)" : "transparent",
              fontFamily: "var(--font-mono)",
              fontSize: 12,
            }}
            onMouseEnter={(e) => {
              if (active !== i)
                (e.currentTarget as HTMLElement).style.background =
                  "var(--bg-elevated)";
            }}
            onMouseLeave={(e) => {
              if (active !== i)
                (e.currentTarget as HTMLElement).style.background =
                  "transparent";
            }}
          >
            <span style={{ color: "var(--text-primary)", fontSize: 11 }}>
              {token.symbol}
            </span>
            <span style={{ textAlign: "right", color: "var(--text-primary)" }}>
              {token.price < 0.01
                ? token.price.toFixed(6)
                : fmt.price(token.price, 2)}
            </span>
            <span
              style={{
                textAlign: "right",
                color: fmt.pctColor(token.change),
                fontSize: 11,
              }}
            >
              {fmt.pct(token.change)}
            </span>
            <span
              style={{
                textAlign: "right",
                color: "var(--text-secondary)",
                fontSize: 10,
              }}
            >
              {fmt.volume(token.volume)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
