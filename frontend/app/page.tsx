'use client';

import Header from '@/components/Header';
import TxTape from '@/components/TxTape';
import WalletRail from '@/components/WalletRail';
import WhaleFeed from '@/components/WhaleFeed';

export default function IntelPage() {
  return (
    <div className="flex flex-col h-screen">
      <Header />
      <main className="flex flex-1 min-h-0 gap-3 p-3">
        <div className="flex flex-col flex-1 min-w-0 gap-3">
          <WhaleFeed />
          <TxTape />
        </div>
        <WalletRail />
      </main>
    </div>
  );
}
