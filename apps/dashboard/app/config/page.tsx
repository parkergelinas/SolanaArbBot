'use client';

import { useCallback, useState } from 'react';

import { CompactPageHeader, DsPanel, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';

type ConfigSection = Record<string, unknown>;

function ConfigSectionEditor({
  sectionKey,
  data,
  onSave,
}: {
  sectionKey: string;
  data: ConfigSection;
  onSave: (section: string, patch: ConfigSection) => Promise<void>;
}) {
  const [fields, setFields] = useState<ConfigSection>(data);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  const handleChange = (key: string, value: string) => {
    const raw: unknown =
      value === 'true' ? true : value === 'false' ? false : isNaN(Number(value)) ? value : Number(value);
    setFields((prev) => ({ ...prev, [key]: raw }));
  };

  const handleSave = async () => {
    setSaving(true);
    setMsg(null);
    try {
      await onSave(sectionKey, fields);
      setMsg({ ok: true, text: 'Saved — config updated and validated.' });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <DsPanel
      compact
      title={sectionKey}
      action={
        <button
          onClick={handleSave}
          disabled={saving}
          className="text-[10px] px-2 py-0.5 rounded-terminal border border-ds-green/30 text-ds-green hover:bg-ds-green/10 disabled:opacity-40 transition-colors"
        >
          {saving ? 'Saving…' : 'Apply'}
        </button>
      }
    >
      <div className="space-y-1.5">
        {Object.entries(fields).map(([key, val]) => (
          <div key={key} className="flex items-center gap-3 py-0.5">
            <label className="font-mono text-[10px] text-ds-text-muted w-44 flex-shrink-0 truncate" title={key}>
              {key}
            </label>
            {typeof val === 'boolean' ? (
              <select
                value={String(val)}
                onChange={(e) => handleChange(key, e.target.value)}
                className="flex-1 bg-ds-elevated border border-ds-border rounded-terminal px-2 py-1 text-[11px] text-ds-text-primary focus:outline-none focus:border-ds-blue/50"
              >
                <option value="true">true</option>
                <option value="false">false</option>
              </select>
            ) : Array.isArray(val) ? (
              <input
                type="text"
                value={(val as string[]).join(', ')}
                readOnly
                className="flex-1 bg-ds-elevated/50 border border-ds-border rounded-terminal px-2 py-1 text-[11px] text-ds-text-muted cursor-not-allowed font-mono"
                title="Array fields require file-level edit"
              />
            ) : (
              <input
                type="text"
                value={String(val)}
                onChange={(e) => handleChange(key, e.target.value)}
                className="flex-1 bg-ds-elevated border border-ds-border rounded-terminal px-2 py-1 text-[11px] text-ds-text-primary focus:outline-none focus:border-ds-blue/50 font-mono"
              />
            )}
          </div>
        ))}
      </div>
      {msg && (
        <p
          className={`text-[10px] mt-2 pt-2 border-t border-ds-border ${
            msg.ok ? 'text-ds-green' : 'text-ds-red'
          }`}
        >
          {msg.text}
        </p>
      )}
    </DsPanel>
  );
}

const EDITABLE_SECTIONS = ['signal_engine', 'features', 'risk', 'execution', 'scalper', 'strategy'];

export default function ConfigPage() {
  const { data: config, loading, refetch } = useFetch(useCallback(() => api.config(), []), 30_000);

  const handleSave = async (section: string, patch: ConfigSection) => {
    await api.patchConfig({ [section]: patch });
    refetch();
  };

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Config"
        subtitle="Edit SystemConfig sections — validated by the risk engine before apply"
        actions={
          config ? (
            <DsBadge tone="muted" mono>
              {EDITABLE_SECTIONS.length} sections
            </DsBadge>
          ) : undefined
        }
      />

      <div className="flex items-center gap-2 px-3 py-2 bg-ds-amber/8 border border-ds-amber/25 rounded-terminal text-[11px] text-ds-amber shrink-0">
        <span className="font-mono uppercase tracking-wider shrink-0">Ephemeral</span>
        <span className="text-ds-text-secondary">
          Changes apply in-process only — edit{' '}
          <code className="font-mono text-ds-amber/90">config.toml</code> for persistence.
        </span>
      </div>

      {loading ? (
        <DsPanel className="text-center py-10">
          <p className="text-ds-text-muted text-[11px]">Loading configuration…</p>
        </DsPanel>
      ) : !config ? (
        <DsPanel className="text-center py-10">
          <p className="text-ds-red text-[11px]">Failed to load config — is the control API running?</p>
        </DsPanel>
      ) : (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-2 flex-1 min-h-0 overflow-y-auto terminal-scroll">
          {EDITABLE_SECTIONS.map((section) => {
            const sectionData = config[section] as ConfigSection | undefined;
            if (!sectionData || typeof sectionData !== 'object') return null;
            return (
              <ConfigSectionEditor key={section} sectionKey={section} data={sectionData} onSave={handleSave} />
            );
          })}
        </div>
      )}
    </PageShell>
  );
}
