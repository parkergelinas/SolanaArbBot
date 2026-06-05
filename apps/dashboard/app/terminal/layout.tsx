import StreamBootstrap from '@/components/terminal/StreamBootstrap';

export default function TerminalLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <StreamBootstrap />
      <div data-terminal className="flex flex-col flex-1 min-h-0 -m-6 overflow-hidden">
        {children}
      </div>
    </>
  );
}
