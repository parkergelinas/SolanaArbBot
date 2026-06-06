export const fmt = {
  price: (n: number, decimals = 4) =>
    n.toLocaleString("en-US", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }),

  pnl: (n: number): string => {
    const sign = n >= 0 ? "+" : "\u2212"; // proper minus U+2212
    return `${sign}$${Math.abs(n).toFixed(2)}`;
  },

  pnlColor: (n: number): string =>
    n > 0 ? "var(--green)" : n < 0 ? "var(--red)" : "var(--text-secondary)",

  size: (n: number) => n.toLocaleString("en-US", { maximumFractionDigits: 3 }),

  volume: (n: number): string => {
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
    if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
    return n.toFixed(0);
  },

  pct: (n: number): string => `${n >= 0 ? "+" : ""}${n.toFixed(2)}%`,

  pctColor: (n: number): string =>
    n > 0 ? "var(--green)" : n < 0 ? "var(--red)" : "var(--text-secondary)",

  timeMs: (ts: number): string => {
    const d = new Date(ts);
    return (
      [
        String(d.getHours()).padStart(2, "0"),
        String(d.getMinutes()).padStart(2, "0"),
        String(d.getSeconds()).padStart(2, "0"),
      ].join(":") +
      "." +
      String(d.getMilliseconds()).padStart(3, "0")
    );
  },

  slot: (n: number): string => n.toLocaleString("en-US"),
};
