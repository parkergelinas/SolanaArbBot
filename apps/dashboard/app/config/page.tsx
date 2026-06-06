'use client';

import { useCallback, useState } from 'react';

import { PageHeader, PageShell } from '@/components/layout/PageShell';
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
    const raw: unknown = value === 'true' ? true : value === 'false' ? false : isNaN(Number(value)) ? value : Number(value);
    setFields(prev => ({ ...prev, [key]: raw }));
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
    <div className="bg-slate-800 border border-slate-700 rounded-xl overflow-hidden">
      <div className="px-4 py-3 border-b border-slate-700 flex items-center justify-between">
        <h3 className="text-sm font-medium text-slate-300 capitalize">
          [{sectionKey}]
        </h3>
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-3 py-1 text-xs rounded-md bg-green-500/10 text-green-400 border border-green-500/30 hover:bg-green-500/20 disabled:opacity-40 transition-colors"
        >
          {saving ? 'Saving…' : 'Apply'}
        </button>
      </div>
      <div className="p-4 space-y-2">
        {Object.entries(fields).map(([key, val]) => (
          <div key={key} className="flex items-center gap-3">
            <label className="mono text-xs text-slate-400 w-48 flex-shrink-0">{key}</label>
            {typeof val === 'boolean' ? (
              <select
                value={String(val)}
                onChange={e => handleChange(key, e.target.value)}
                className="flex-1 bg-slate-900 border border-slate-600 rounded px-2 py-1 text-xs text-slate-200 focus:outline-none focus:border-green-500"
              >
                <option value="true">true</option>
                <option value="false">false</option>
              </select>
            ) : Array.isArray(val) ? (
              <input
                type="text"
                value={(val as string[]).join(', ')}
                readOnly
                className="flex-1 bg-slate-900/50 border border-slate-700 rounded px-2 py-1 text-xs text-slate-500 cursor-not-allowed"
                title="Array fields require file-level edit"
              />
            ) : (
              <input
                type="text"
                value={String(val)}
                onChange={e => handleChange(key, e.target.value)}
                className="flex-1 bg-slate-900 border border-slate-600 rounded px-2 py-1 text-xs text-slate-200 focus:outline-none focus:border-green-500 mono"
              />
            )}
          </div>
        ))}
      </div>
      {msg && (
        <div className={`px-4 py-2 text-xs border-t border-slate-700 ${msg.ok ? 'text-green-400' : 'text-red-400'}`}>
          {msg.text}
        </div>
      )}
    </div>
  );
}

const EDITABLE_SECTIONS = ['signal_engine', 'features', 'risk', 'execution'];

export default function ConfigPage() {
  const { data: config, loading, refetch } = useFetch(
    useCallback(() => api.config(), []),
    30_000,
  );

  const handleSave = async (section: string, patch: ConfigSection) => {
    await api.patchConfig({ [section]: patch });
    refetch();
  };

  return (
    <PageShell className="max-w-4xl">
      <PageHeader
        title="Configuration"
        description="Edit SystemConfig sections — all changes are validated by the risk engine before being applied."
      />

      <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg px-4 py-3 text-amber-300 text-xs">
        ⚠ Config changes take effect immediately in the running process but are not persisted to disk.
        Edit <code className="bg-amber-900/30 px-1 rounded">config.toml</code> for permanent changes.
      </div>

      {loading ? (
        <div className="text-slate-500 text-sm py-8 text-center">Loading configuration…</div>
      ) : !config ? (
        <div className="text-red-400 text-sm py-8 text-center">Failed to load config — is the control API running?</div>
      ) : (
        <div className="space-y-4">
          {EDITABLE_SECTIONS.map(section => {
            const sectionData = config[section] as ConfigSection | undefined;
            if (!sectionData || typeof sectionData !== 'object') return null;
            return (
              <ConfigSectionEditor
                key={section}
                sectionKey={section}
                data={sectionData}
                onSave={handleSave}
              />
            );
          })}
        </div>
      )}
    </PageShell>
  );
}
