// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port, and suggests using 1420
  server: {
    port: 1420,
    strictPort: true,
  },
  // Fix hot reload in Tauri development
  clearScreen: false,
  // Enable Tauri integration
  build: {
    target: 'esnext',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      external: [
        '@tauri-apps/api/core',
        '@tauri-apps/api',
        '@tauri-apps/plugin-store',
        '@tauri-apps/plugin-dialog'
      ]
    }
  },
  // Add path aliasing for improved imports
  resolve: {
    alias: {
      '@': resolve(__dirname, './src')
    },
  }
});