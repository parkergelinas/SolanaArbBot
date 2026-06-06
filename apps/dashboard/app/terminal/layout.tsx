import PaperBootstrap from '@/components/paper/PaperBootstrap';
import DemoBootstrap from '@/components/terminal/DemoBootstrap';
import PriceBootstrap from '@/components/terminal/PriceBootstrap';
import WatchlistBootstrap from '@/components/terminal/WatchlistBootstrap';
import { DexScreenerProvider } from '@/components/terminal/DexScreenerProvider';
import LiveHubBootstrap from '@/components/terminal/LiveHubBootstrap';
import StreamBootstrap from '@/components/terminal/StreamBootstrap';
import TerminalErrorBoundary from '@/components/terminal/TerminalErrorBoundary';
import { isTerminalDemoEnabled } from '@/lib/config/env';

export default function TerminalLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <StreamBootstrap />
      <WatchlistBootstrap />
      <PriceBootstrap />
      <LiveHubBootstrap />
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
