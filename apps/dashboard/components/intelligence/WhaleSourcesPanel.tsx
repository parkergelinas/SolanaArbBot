'use client';

import { useState } from 'react';

import {
  WHALE_DATA_SOURCES,
  WHALE_DETECTION_METHODS,
  WALLET_TIER_RULES,
  WHALE_THRESHOLD_SOL,
} from '@/lib/intelligence/whaleSources';
import { useStreamStore } from '@/stores/streamStore';
import { useIntelConnected } from '@/lib/intelligence/hooks';
import { useWsContext } from '@/components/WebSocketProvider';

function SourceStatus({ live, label }: { live: boolean; label: string }) {
  return (
    <span
      className={`inline-flex items-center gap-1 text-[9px] font-mono uppercase px-1.5 py-px border rounded-terminal ${
        live
          ? 'text-ds-green border-ds-green/30'
          : 'text-ds-text-muted border-ds-border'
      }`}
    >
      <span className={`w-1 h-1 rounded-full ${live ? 'bg-ds-green' : 'bg-ds-text-muted'}`} />
      {live ? 'live' : 'offline'}
      <span className="text-ds-text-muted normal-case">· {label}</span>
    </span>
  );
}

export default function WhaleSourcesPanel({ compact = false }: { compact?: boolean }) {
  const [open, setOpen] = useState(false);
  const intelConnected = useIntelConnected();
  const { connected: controlConnected } = useWsContext();
  const streamConnected = useStreamStore((s) => s.connected);

  const liveMap: Record<string, boolean> = {
    'intelligence-api': intelConnected,
    'stream-api': streamConnected,
    'control-api': controlConnected,
    dexscreener: true,
  };

  return (
    <section className="bg-ds-surface border border-ds-border rounded-terminal overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={`w-full flex items-center justify-between text-left hover:bg-ds-elevated/40 transition-colors ${
          compact ? 'px-3 py-1.5 gap-2' : 'px-4 py-2.5'
        }`}
      >
        <div className="min-w-0">
          <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
            {compact ? 'Whale Sources' : 'Whale Tracking · Sources & Methods'}
          </span>
          <p className={`text-ds-text-secondary truncate ${compact ? 'text-[10px] mt-0' : 'text-[11px] mt-0.5'}`}>
            {WHALE_THRESHOLD_SOL} SOL · Yellowstone → WS
          </p>
        </div>
        <span className="text-ds-text-muted text-xs shrink-0">{open ? '▾' : '▸'}</span>
      </button>

      {open && (
        <div
          className={`border-t border-ds-border space-y-3 text-[11px] ${
            compact ? 'px-3 py-2 max-h-[min(24rem,50vh)] overflow-y-auto terminal-scroll' : 'px-4 py-3 space-y-4'
          }`}
        >
          <div className={`grid gap-2 ${compact ? 'grid-cols-1' : 'grid-cols-1 md:grid-cols-2 gap-3'}`}>
            {WHALE_DATA_SOURCES.map((src) => (
              <div
                key={src.id}
                className="border border-ds-border rounded-terminal p-3 bg-ds-elevated/30"
              >
                <div className="flex items-center justify-between gap-2 mb-1">
                  <span className="font-semibold text-ds-text-primary">{src.label}</span>
                  <SourceStatus live={liveMap[src.id] ?? false} label={src.id} />
                </div>
                <p className="text-ds-text-secondary leading-relaxed">{src.role}</p>
                <p className="font-mono text-[10px] text-ds-text-muted mt-1 truncate" title={src.endpoint}>
                  {src.endpoint}
                </p>
                {src.signals.length > 0 && (
                  <p className="text-[10px] text-ds-blue mt-1.5">
                    {src.signals.join(' · ')}
                  </p>
                )}
              </div>
            ))}
          </div>

          <div>
            <p className="text-[10px] uppercase tracking-wider text-ds-text-muted mb-2">
              Detection methods
            </p>
            <ul className="space-y-1.5">
              {WHALE_DETECTION_METHODS.map((m) => (
                <li key={m.name} className="text-ds-text-secondary">
                  <span className="text-ds-text-primary font-medium">{m.name}</span>
                  {' — '}
                  {m.description}
                </li>
              ))}
            </ul>
          </div>

          <div>
            <p className="text-[10px] uppercase tracking-wider text-ds-text-muted mb-2">
              Wallet tiers
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {WALLET_TIER_RULES.map((rule) => (
                <div
                  key={rule.tier}
                  className="border border-ds-border rounded-terminal px-2 py-1.5"
                >
                  <p className="font-semibold text-ds-text-primary">{rule.label}</p>
                  <p className="text-[10px] text-ds-text-muted mt-0.5 leading-snug">
                    {rule.criteria}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
