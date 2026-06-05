import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './pages/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
    './app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        platform: {
          bg: '#0b0e14',
          surface: '#131720',
          elevated: '#1a1f2e',
          border: '#252b3b',
          'border-hover': '#323a4f',
          muted: '#8b95a8',
          accent: '#00dfa8',
          'accent-dim': '#00dfa820',
          blue: '#4da3ff',
          purple: '#a78bfa',
          amber: '#fbbf24',
        },
        brand: {
          50:  '#f0fdf4',
          400: '#4ade80',
          500: '#22c55e',
          600: '#16a34a',
        },
        terminal: {
          bg: '#060d18',
          panel: '#0a1628',
          border: '#1e3a4a',
          hover: '#0f2035',
          live: '#22d3ee',
          muted: '#64748b',
        },
        flow: {
          buy: '#22c55e',
          sell: '#ef4444',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
};

export default config;
