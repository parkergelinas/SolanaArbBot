import { fmt } from "@/lib/formatters";

interface Props {
  status: {
    strategy: string;
    solBalance: number;
    slot: number;
    dryRun: boolean;
  };
  connected: boolean;
}

export default function Header({ status, connected }: Props) {
  return (
    <div
      style={{
        height: 40,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 16px",
        gap: 24,
      }}
    >
      {/* Left */}
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontWeight: 600,
            fontSize: 13,
            letterSpacing: "0.05em",
            color: "var(--text-primary)",
          }}
        >
          SOLANA ARB
        </span>
        <Badge label={status.strategy} variant="active" />
        {status.dryRun && <Badge label="DRY RUN" variant="warning" />}
      </div>

      {/* Center: slot */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--text-secondary)",
        }}
      >
        SLOT {fmt.slot(status.slot)}
      </div>

      {/* Right */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 16,
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--text-secondary)",
        }}
      >
        <span>{fmt.price(status.solBalance, 4)} SOL</span>
        <Clock />
      </div>
    </div>
  );
}

function Badge({
  label,
  variant,
}: {
  label: string;
  variant: "active" | "warning";
}) {
  const color = variant === "active" ? "var(--green)" : "var(--amber)";
  const bg = variant === "active" ? "var(--green-dim)" : "transparent";
  return (
    <span
      style={{
        fontSize: 10,
        fontWeight: 500,
        padding: "2px 7px",
        border: `1px solid ${color}`,
        color,
        background: bg,
        borderRadius: "var(--radius)",
        letterSpacing: "0.06em",
      }}
    >
      {label}
    </span>
  );
}

function Clock() {
  "use client";
  const { useState, useEffect } = require("react");
  const [time, setTime] = useState("");
  useEffect(() => {
    const tick = () => setTime(new Date().toUTCString().split(" ")[4] + " UTC");
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);
  return <span>{time}</span>;
}
