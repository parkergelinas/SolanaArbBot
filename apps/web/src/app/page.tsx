"use client";
import { useBotData } from "@/hooks/useBotData";
import Header from "@/components/Header";
import Orderbook from "@/components/Orderbook";
import Watchlist from "@/components/Watchlist";
import Positions from "@/components/Positions";
import TradeLog from "@/components/TradeLog";
import StatusBar from "@/components/StatusBar";

export default function Terminal() {
  const data = useBotData();

  return (
    <div
      style={{
        display: "grid",
        gridTemplateRows: "40px 1fr 28px",
        gridTemplateColumns: "280px 1fr 320px",
        height: "100vh",
        width: "100vw",
        overflow: "hidden",
        background: "var(--bg-base)",
      }}
    >
      {/* Header — full width */}
      <div
        style={{
          gridColumn: "1 / -1",
          borderBottom: "1px solid var(--bg-border)",
        }}
      >
        <Header status={data.status} connected={data.connected} />
      </div>

      {/* Left: Watchlist */}
      <div
        style={{
          borderRight: "1px solid var(--bg-border)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <Watchlist tokens={data.watchlist} />
      </div>

      {/* Center: Orderbook + TradeLog */}
      <div
        style={{
          display: "grid",
          gridTemplateRows: "1fr 200px",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            borderBottom: "1px solid var(--bg-border)",
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <Orderbook data={data.orderbook} />
        </div>
        <div
          style={{
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <TradeLog trades={data.trades} />
        </div>
      </div>

      {/* Right: Positions */}
      <div
        style={{
          borderLeft: "1px solid var(--bg-border)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <Positions positions={data.positions} />
      </div>

      {/* Status bar — full width */}
      <div
        style={{
          gridColumn: "1 / -1",
          borderTop: "1px solid var(--bg-border)",
        }}
      >
        <StatusBar status={data.status} connected={data.connected} />
      </div>
    </div>
  );
}
