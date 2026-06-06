import { fmt } from "@/lib/formatters";

interface Level {
  price: number;
  size: number;
  depth: number;
}
interface Props {
  data: {
    pair: string;
    asks: Level[];
    bids: Level[];
    spread: number;
    spreadPct: number;
  };
}

export default function Orderbook({ data }: Props) {
  const { pair, asks, bids, spread, spreadPct } = data;
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
        <span>ORDERBOOK</span>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>
          {pair}
        </span>
      </div>
      {/* Column headers */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr 60px",
          padding: "4px 12px",
          fontSize: 10,
          color: "var(--text-muted)",
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          flexShrink: 0,
        }}
      >
        <span>SIZE</span>
        <span style={{ textAlign: "right" }}>PRICE</span>
        <span style={{ textAlign: "right" }}>DEPTH</span>
      </div>
      {/* Asks (reversed — lowest ask at bottom, closest to mid) */}
      {[...asks].reverse().map((a, i) => (
        <OBRow key={i} level={a} side="ask" />
      ))}
      {/* Spread */}
      <div
        style={{
          textAlign: "center",
          padding: "3px 0",
          fontSize: 11,
          color: spreadPct > 0.1 ? "var(--amber)" : "var(--text-secondary)",
          borderTop: "1px solid var(--bg-border)",
          borderBottom: "1px solid var(--bg-border)",
          flexShrink: 0,
          fontFamily: "var(--font-mono)",
        }}
      >
        {fmt.price(spread, 4)} ({fmt.pct(spreadPct)}) spread
      </div>
      {/* Bids */}
      {bids.map((b, i) => (
        <OBRow key={i} level={b} side="bid" />
      ))}
    </div>
  );
}

function OBRow({ level, side }: { level: Level; side: "bid" | "ask" }) {
  const isBid = side === "bid";
  return (
    <div
      style={{
        position: "relative",
        height: 22,
        display: "grid",
        gridTemplateColumns: "1fr 1fr 60px",
        alignItems: "center",
        padding: "0 12px",
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        flexShrink: 0,
      }}
    >
      {/* Depth bar */}
      <div
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          right: 0,
          width: `${level.depth * 100}%`,
          background: isBid ? "var(--green-dim)" : "var(--red-dim)",
          zIndex: 0,
        }}
      />
      <span
        style={{
          position: "relative",
          zIndex: 1,
          color: "var(--text-primary)",
        }}
      >
        {fmt.size(level.size)}
      </span>
      <span
        style={{
          position: "relative",
          zIndex: 1,
          textAlign: "right",
          color: isBid ? "var(--green)" : "var(--red)",
        }}
      >
        {fmt.price(level.price, 2)}
      </span>
      <span
        style={{
          position: "relative",
          zIndex: 1,
          textAlign: "right",
          color: "var(--text-secondary)",
          fontSize: 10,
        }}
      >
        {(level.depth * 100).toFixed(0)}%
      </span>
    </div>
  );
}
