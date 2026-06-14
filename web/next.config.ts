import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Server components read project artifacts from the repo root (web/..).
  // Keep the app strictly server-rendered for data fidelity; no static fixtures.
  experimental: {
    // allow reading files outside web/ at request time
  },
};

export default nextConfig;
