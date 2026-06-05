/** @type {import('next').NextConfig} */
const nextConfig = {
  // Proxy REST to the Rust control-api when CONTROL_API_URL is set (Vercel env).
  // Do NOT default to localhost on Vercel — that host does not exist in production.
  async rewrites() {
    const api = process.env.CONTROL_API_URL?.replace(/\/$/, '');
    if (!api) {
      return [];
    }
    return [
      {
        source: '/api/:path*',
        destination: `${api}/api/:path*`,
      },
    ];
  },
};

module.exports = nextConfig;
