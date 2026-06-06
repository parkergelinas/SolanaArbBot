import PaperBootstrap from '@/components/paper/PaperBootstrap';
import DemoBootstrap from '@/components/terminal/DemoBootstrap';
import PriceBootstrap from '@/components/terminal/PriceBootstrap';
import WatchlistBootstrap from '@/components/terminal/WatchlistBootstrap';
import { DexScreenerProvider } from '@/components/terminal/DexScreenerProvider';
import TerminalErrorBoundary from '@/components/terminal/TerminalErrorBoundary';
import { isTerminalDemoEnabled } from '@/lib/config/env';

export default function TerminalLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <WatchlistBootstrap />
      <PriceBootstrap />
      {isTerminalDemoEnabled() && <DemoBootstrap />}
      <PaperBootstrap />
      <DexScreenerProvider>
        <TerminalErrorBoundary label="Terminal">
          <div data-terminal className="flex flex-col flex-1 min-h-0 overflow-hidden">
            {children}
          </div>
        </TerminalErrorBoundary>
      </DexScreenerProvider>
    </>
  );
}
