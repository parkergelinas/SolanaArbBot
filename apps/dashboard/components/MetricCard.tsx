interface MetricCardProps {
  label: string;
  value: React.ReactNode;
  sub?: string;
  accent?: 'green' | 'red' | 'blue' | 'yellow' | 'purple' | 'default';
}

const accentMap: Record<string, string> = {
  green:   'text-green-400',
  red:     'text-red-400',
  blue:    'text-blue-400',
  yellow:  'text-yellow-400',
  purple:  'text-purple-400',
  default: 'text-slate-100',
};

export default function MetricCard({ label, value, sub, accent = 'default' }: MetricCardProps) {
  return (
    <div className="bg-slate-800 border border-slate-700 rounded-xl p-4">
      <p className="text-slate-400 text-xs uppercase tracking-wider mb-2">{label}</p>
      <p className={`text-2xl font-semibold mono ${accentMap[accent]}`}>{value}</p>
      {sub && <p className="text-slate-500 text-xs mt-1">{sub}</p>}
    </div>
  );
}
