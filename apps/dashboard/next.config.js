/** @type {import('next').NextConfig} */
const nextConfig = {
  // Proxy REST to the Rust control-api when CONTROL_API_URL is set (Vercel env).
  // Do NOT default to localhost on Vercel — that host does not exist in production.
  async rewrites() {
    const api = process.env.CONTROL_API_URL?.replace(/\/$/, '');
    const streamHttp =
      process.env.NEXT_PUBLIC_STREAM_HTTP_URL?.replace(/\/$/, '') ||
      'http://localhost:8080';
    const rules = [];
    // Scanner REST lives on stream-api, not control-api.
    rules.push({
      source: '/api/scanners/:path*',
      destination: `${streamHttp}/api/scanners/:path*`,
    });
    if (api) {
      rules.push({
        source: '/api/:path*',
        destination: `${api}/api/:path*`,
      });
    }
    return rules;
  },
};

module.exports = nextConfig;
