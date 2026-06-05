import StreamBootstrap from '@/components/terminal/StreamBootstrap';

export default function TerminalLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <StreamBootstrap />
      <div className="flex flex-col h-full min-h-0 -m-6">{children}</div>
    </>
  );
}
