import type { Config } from 'tailwindcss';

const config: Config = {
  content: ['./app/**/*.{js,ts,jsx,tsx}', './components/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        platform: {
          bg: '#0b0e14',
          surface: '#131720',
          elevated: '#1a1f2e',
          border: '#252b3b',
          muted: '#8b95a8',
          accent: '#00dfa8',
        },
      },
    },
  },
  plugins: [],
};
export default config;
