/** @type {import('next').NextConfig} */
const nextConfig = {
  // Proxy REST to the Rust control-api (server-side env on Vercel).
  // WebSocket still uses NEXT_PUBLIC_WS_URL — point at your deployed API host.
  async rewrites() {
    const api = process.env.CONTROL_API_URL || 'http://localhost:3001';
    return [
      {
        source: '/api/:path*',
        destination: `${api}/api/:path*`,
      },
    ];
  },
};

module.exports = nextConfig;
