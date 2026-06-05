/** @type {import('next').NextConfig} */
const nextConfig = {
  // Allow the dashboard to call the control-api at runtime
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${process.env.CONTROL_API_URL || 'http://localhost:3001'}/api/:path*`,
      },
    ];
  },
};

module.exports = nextConfig;
