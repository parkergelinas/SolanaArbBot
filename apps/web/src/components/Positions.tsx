import { fmt } from "@/lib/formatters";

interface Position {
  pair: string;
  size: number;
  entry: number;
  pnl: number;
  strategy: string;
}
interface Props {
  positions: Position[];
}

export default function Positions({ positions }: Props) {
  const totalPnl = positions.reduce((s, p) => s + p.pnl, 0);

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
        <span>POSITIONS</span>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>
          {positions.length} open
        </span>
      </div>
      {/* Column headers */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 60px 70px 70px",
          padding: "4px 12px",
          fontSize: 10,
          color: "var(--text-muted)",
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          flexShrink: 0,
        }}
      >
        <span>PAIR</span>
        <span style={{ textAlign: "right" }}>SIZE</span>
        <span style={{ textAlign: "right" }}>ENTRY</span>
        <span style={{ textAlign: "right" }}>PNL</span>
      </div>
      {/* Rows */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {positions.length === 0 ? (
          <div
            style={{
              padding: 12,
              color: "var(--text-muted)",
              fontSize: 12,
              fontFamily: "var(--font-mono)",
            }}
          >
            No open positions.
          </div>
        ) : (
          positions.map((p, i) => (
            <div
              key={i}
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 60px 70px 70px",
                height: 26,
                alignItems: "center",
                padding: "0 12px",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
              }}
            >
              <span style={{ color: "var(--text-primary)", fontSize: 11 }}>
                {p.pair}
              </span>
              <span
                style={{ textAlign: "right", color: "var(--text-secondary)" }}
              >
                {fmt.size(p.size)}
              </span>
              <span
                style={{ textAlign: "right", color: "var(--text-secondary)" }}
              >
                {fmt.price(p.entry, 2)}
              </span>
              <span style={{ textAlign: "right", color: fmt.pnlColor(p.pnl) }}>
                {fmt.pnl(p.pnl)}
              </span>
            </div>
          ))
        )}
      </div>
      {/* Summary */}
      {positions.length > 0 && (
        <div
          style={{
            borderTop: "1px solid var(--bg-border)",
            padding: "6px 12px",
            display: "flex",
            justifyContent: "space-between",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            flexShrink: 0,
          }}
        >
          <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>
            TOTAL
          </span>
          <span style={{ color: fmt.pnlColor(totalPnl), fontWeight: 500 }}>
            {fmt.pnl(totalPnl)}
          </span>
        </div>
      )}
    </div>
  );
}
