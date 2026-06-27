import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// В dev API control-plane проксируется на backend:8080 (или localhost).
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: process.env.VITE_API_TARGET ?? "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
});
