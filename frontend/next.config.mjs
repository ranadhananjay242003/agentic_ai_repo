/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  async rewrites() {
    return [
      {
        // 1. Change source to '/service/...' to avoid Next.js '/api' conflicts
        source: '/service/:path*',
        // 2. Keep destination pointing to your Render Backend
        destination: 'https://nexus-orchestrator-vad4.onrender.com/api/v1/:path*',
      },
    ]
  },
};

export default nextConfig;