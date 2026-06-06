"use client";
import { fmt } from "@/lib/formatters";
import { useRef, useEffect } from "react";

interface Trade {
  id: number;
  ts: number;
  strategy: string;
  action: string;
  pair: string;
  size: number;
  price: number;
  pnl: number | null;
  status: string;
}
interface Props {
  trades: Trade[];
}

export default function TradeLog({ trades }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [trades.length]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="panel-header">
        <span>TRADE LOG</span>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--text-muted)",
          }}
        >
          {trades.length}
        </span>
      </div>
      <div
        style={{
          flex: 1,
          overflowY: "auto",
          fontFamily: "var(--font-mono)",
          fontSize: 11,
        }}
      >
        {trades.map((t) => (
          <TradeRow key={t.id} trade={t} />
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}

function TradeRow({ trade: t }: { trade: Trade }) {
  const isReverted = t.status === "reverted";
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "88px 60px 42px 90px 60px 72px 72px",
        height: 22,
        alignItems: "center",
        padding: "0 12px",
        opacity: isReverted ? 0.5 : 1,
        textDecoration: isReverted ? "line-through" : "none",
      }}
    >
      <span style={{ color: "var(--text-muted)" }}>{fmt.timeMs(t.ts)}</span>
      <span style={{ color: "var(--text-secondary)" }}>{t.strategy}</span>
      <span
        style={{
          color: t.action === "BUY" ? "var(--blue)" : "var(--text-secondary)",
        }}
      >
        {t.action}
      </span>
      <span style={{ color: "var(--text-primary)" }}>{t.pair}</span>
      <span style={{ color: "var(--text-primary)", textAlign: "right" }}>
        {fmt.size(t.size)}
      </span>
      <span style={{ color: "var(--text-primary)", textAlign: "right" }}>
        {fmt.price(t.price, 2)}
      </span>
      <span
        style={{
          textAlign: "right",
          color:
            t.status === "pending"
              ? "var(--amber)"
              : t.pnl === null
                ? "var(--text-muted)"
                : fmt.pnlColor(t.pnl),
        }}
      >
        {t.status === "pending"
          ? "pending"
          : t.pnl === null
            ? "—"
            : fmt.pnl(t.pnl)}
      </span>
    </div>
  );
}
