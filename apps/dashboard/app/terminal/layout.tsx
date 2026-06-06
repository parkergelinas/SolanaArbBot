import DemoBootstrap from '@/components/terminal/DemoBootstrap';
import { DexScreenerProvider } from '@/components/terminal/DexScreenerProvider';
import LiveHubBootstrap from '@/components/terminal/LiveHubBootstrap';
import StreamBootstrap from '@/components/terminal/StreamBootstrap';
import TerminalErrorBoundary from '@/components/terminal/TerminalErrorBoundary';

export default function TerminalLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <StreamBootstrap />
      <LiveHubBootstrap />
      <DemoBootstrap />
      <DexScreenerProvider>
        <TerminalErrorBoundary label="Terminal">
          <div data-terminal className="flex flex-col flex-1 min-h-0 -m-6 overflow-hidden">
            {children}
          </div>
        </TerminalErrorBoundary>
      </DexScreenerProvider>
    </>
  );
}
