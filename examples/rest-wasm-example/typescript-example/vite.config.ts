import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import { resolve } from 'path';
import { wasmPack } from './vite-plugin-wasm-pack';

export default defineConfig({
  plugins: [
    wasmPack({
      cratePath: '../rest-api',
      outDir: 'public/pkg',
      features: ['wasm-client'],
    }),
    solid(),
  ],
  resolve: {
    alias: {
      '@wasm': resolve(__dirname, './public/pkg'),
    },
  },
  server: {
    port: 3001,
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      }
    },
    fs: {
      // Allow serving files from public directory
      allow: ['..'],
    },
  },
  build: {
    target: 'esnext',
  },
  optimizeDeps: {
    exclude: ['@wasm/rest_api.js'],
  },
});