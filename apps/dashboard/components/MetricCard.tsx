interface MetricCardProps {
  label: string;
  value: React.ReactNode;
  sub?: string;
  accent?: 'green' | 'red' | 'blue' | 'yellow' | 'purple' | 'default';
}

const accentMap: Record<string, string> = {
  green: 'text-ds-green',
  red: 'text-ds-red',
  blue: 'text-ds-blue',
  yellow: 'text-ds-amber',
  purple: 'text-ds-blue',
  default: 'text-ds-text-primary',
};

export default function MetricCard({ label, value, sub, accent = 'default' }: MetricCardProps) {
  return (
    <div className="bg-ds-surface border border-ds-border rounded-terminal px-3 py-2.5">
      <p className="text-[9px] uppercase tracking-[0.12em] text-ds-text-muted mb-0.5">{label}</p>
      <p className={`text-[15px] font-semibold font-mono tabular-nums ${accentMap[accent]}`}>{value}</p>
      {sub && <p className="text-ds-text-muted text-[10px] mt-0.5">{sub}</p>}
    </div>
  );
}
