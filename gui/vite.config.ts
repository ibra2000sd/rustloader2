import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port, and suggests using 1420
  // Feel free to change this if needed
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
  },
});