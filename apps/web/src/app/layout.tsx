import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SolanaArbBot",
  description: "Personal Solana arbitrage terminal",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
