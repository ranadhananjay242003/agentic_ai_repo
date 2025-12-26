/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  async rewrites() {
    return [
      {
        // Capture any request to /api/v1/...
        source: '/api/v1/:path*',
        // Proxy it to Render
        destination: `${process.env.ORCHESTRATOR_URL}/api/v1/:path*`,
      },
    ]
  },
};

export default nextConfig;