'use client';

import dynamic from 'next/dynamic';

const WalletMultiButton = dynamic(
  async () => (await import('@solana/wallet-adapter-react-ui')).WalletMultiButton,
  { ssr: false },
);

export default function WalletConnectButton() {
  return (
    <WalletMultiButton
      className="!bg-platform-accent/15 !border !border-platform-accent/40 !text-platform-accent !text-xs !h-8 !rounded-lg !font-medium hover:!bg-platform-accent/25 !transition-colors"
    />
  );
}
