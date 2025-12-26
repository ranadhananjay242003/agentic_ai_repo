/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  async rewrites() {
    return [
      {
        source: '/api/v1/:path*',
        // HARDCODED URL to rule out environment variable issues
        destination: 'https://nexus-orchestrator-vad4.onrender.com/api/v1/:path*',
      },
    ]
  },
};

export default nextConfig;