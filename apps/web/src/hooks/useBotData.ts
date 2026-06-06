"use client";
import { useState, useEffect, useRef, useCallback } from "react";
import {
  MOCK_ORDERBOOK,
  MOCK_WATCHLIST,
  MOCK_POSITIONS,
  MOCK_TRADES,
  MOCK_STATUS,
} from "@/lib/mock-data";

const WS_URL = process.env.NEXT_PUBLIC_BOT_WS_URL;

export function useBotData() {
  const [orderbook, setOrderbook] = useState(MOCK_ORDERBOOK);
  const [watchlist, setWatchlist] = useState(MOCK_WATCHLIST);
  const [positions, setPositions] = useState(MOCK_POSITIONS);
  const [trades, setTrades] = useState(MOCK_TRADES);
  const [status, setStatus] = useState(MOCK_STATUS);
  const [connected, setConnected] = useState(false);
  const ws = useRef<WebSocket | null>(null);

  const connect = useCallback(() => {
    if (!WS_URL) {
      // No backend — run on mock data indefinitely
      setConnected(false);
      return;
    }
    const socket = new WebSocket(WS_URL);
    ws.current = socket;

    socket.onopen = () => setConnected(true);
    socket.onclose = () => {
      setConnected(false);
      setTimeout(connect, 3000); // auto-reconnect
    };
    socket.onerror = () => socket.close();
    socket.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);
        if (msg.type === "orderbook") setOrderbook(msg.data);
        if (msg.type === "watchlist") setWatchlist(msg.data);
        if (msg.type === "positions") setPositions(msg.data);
        if (msg.type === "trade")
          setTrades((p) => [msg.data, ...p].slice(0, 500));
        if (msg.type === "status") setStatus(msg.data);
      } catch {}
    };
  }, []);

  useEffect(() => {
    connect();
    return () => ws.current?.close();
  }, [connect]);

  return { orderbook, watchlist, positions, trades, status, connected };
}
