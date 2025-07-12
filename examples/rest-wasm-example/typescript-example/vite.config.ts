import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import { resolve } from "path";
import { heyApiPlugin } from "@hey-api/vite-plugin";

export default defineConfig({
  plugins: [heyApiPlugin({}), solid()],
  resolve: {
    alias: {},
  },
  server: {
    port: 3001,
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
    fs: {
      // Allow serving files from public directory
      allow: [".."],
    },
  },
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: [],
  },
});
