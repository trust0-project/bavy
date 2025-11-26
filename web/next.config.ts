import type { NextConfig } from "next";

const nextConfig: NextConfig = {

  env: {
    NEXT_PUBLIC_RELAY_URL: process.env.NEXT_PUBLIC_RELAY_URL,
  },
  
  // Disable image optimization if needed, or keep it enabled for standalone
  images: {
    unoptimized: true,
  },
};

export default nextConfig;
