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
        ds: {
          base: 'var(--bg-base)',
          surface: 'var(--bg-surface)',
          elevated: 'var(--bg-elevated)',
          border: 'var(--bg-border)',
          'text-primary': 'var(--text-primary)',
          'text-secondary': 'var(--text-secondary)',
          'text-muted': 'var(--text-muted)',
          green: 'var(--green)',
          red: 'var(--red)',
          amber: 'var(--amber)',
          blue: 'var(--blue)',
          'green-dim': 'var(--green-dim)',
          'red-dim': 'var(--red-dim)',
        },
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
          50: '#f0fdf4',
          400: '#4ade80',
          500: '#22c55e',
          600: '#16a34a',
        },
      },
      fontFamily: {
        mono: ['var(--font-mono)'],
        ui: ['var(--font-ui)'],
      },
      borderRadius: {
        terminal: '2px',
      },
    },
  },
  plugins: [],
};

export default config;
