"use client";
import { fmt } from "@/lib/formatters";

interface Props {
  status: { slot: number; latencyMs: number; jitoOk: boolean; dryRun: boolean };
  connected: boolean;
}

export default function StatusBar({ status, connected }: Props) {
  const latColor =
    status.latencyMs < 100
      ? "var(--green)"
      : status.latencyMs < 300
        ? "var(--amber)"
        : "var(--red)";

  return (
    <div
      style={{
        height: 28,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 12px",
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        color: "var(--text-secondary)",
      }}
    >
      {/* Left */}
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <ConnDot connected={connected} />
        <span>
          Slot{" "}
          <span style={{ color: "var(--text-primary)" }}>
            {fmt.slot(status.slot)}
          </span>
        </span>
        <span>
          Latency <span style={{ color: latColor }}>{status.latencyMs}ms</span>
        </span>
        <span>
          Jito{" "}
          <span
            style={{ color: status.jitoOk ? "var(--green)" : "var(--red)" }}
          >
            {status.jitoOk ? "✓" : "✗"}
          </span>
        </span>
      </div>

      {/* Right */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        {status.dryRun && (
          <span
            style={{
              color: "var(--amber)",
              border: "1px solid var(--amber)",
              padding: "1px 8px",
              borderRadius: "var(--radius)",
              fontSize: 10,
            }}
          >
            DRY RUN
          </span>
        )}
        <HaltButton />
      </div>
    </div>
  );
}

function ConnDot({ connected }: { connected: boolean }) {
  return (
    <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: "50%",
          display: "inline-block",
          background: connected ? "var(--green)" : "var(--red)",
          animation: connected
            ? "none"
            : "pulse 1s ease-in-out infinite alternate",
        }}
      />
      <style>{`@keyframes pulse { from { opacity: 0.3; } to { opacity: 1; } }`}</style>
      <span>{connected ? "CONNECTED" : "DISCONNECTED"}</span>
    </span>
  );
}

function HaltButton() {
  return (
    <button
      onClick={() => {
        if (confirm("Send halt signal to bot?")) {
          fetch("/api/bot/halt", { method: "POST" }).catch(() => {});
        }
      }}
      style={{
        color: "var(--red)",
        border: "1px solid var(--red)",
        background: "transparent",
        padding: "2px 10px",
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        borderRadius: "var(--radius)",
        cursor: "pointer",
        letterSpacing: "0.05em",
      }}
      onMouseEnter={(e) => {
        const el = e.currentTarget as HTMLButtonElement;
        el.style.background = "var(--red)";
        el.style.color = "var(--bg-base)";
      }}
      onMouseLeave={(e) => {
        const el = e.currentTarget as HTMLButtonElement;
        el.style.background = "transparent";
        el.style.color = "var(--red)";
      }}
    >
      ■ HALT
    </button>
  );
}
