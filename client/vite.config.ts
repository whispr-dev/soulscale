import { defineConfig } from "vite";

// The Rust server (soulscale-server) serves the built client in production. In dev,
// `npm run dev` serves the client with hot reload and proxies the API/WebSocket to the
// server on :3001. All client URLs are document-relative, so the same build works when
// Discord serves it from "/" or from "/.proxy/".
export default defineConfig({
  base: "./",
  build: { target: "es2022", assetsInlineLimit: 0, chunkSizeWarningLimit: 900 },
  server: {
    port: 5173,
    // Allow Cloudflare quick tunnels when testing inside Discord against the dev server.
    allowedHosts: [".trycloudflare.com"],
    proxy: {
      "/api": "http://127.0.0.1:3001",
      "/ws": { target: "ws://127.0.0.1:3001", ws: true },
    },
  },
});
