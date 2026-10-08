import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { visualizer } from 'rollup-plugin-visualizer';
import path from 'node:path';

const isAnalyze = process.env.ANALYZE === 'true';

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    ...(isAnalyze
      ? [
          visualizer({
            filename: 'dist/stats.html',
            open: false,
            gzipSize: true,
            brotliSize: true,
            title: 'LynxSearch Frontend Bundle Stats',
          }),
        ]
      : []),
  ],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  envDir: '../../',
  clearScreen: false,
  build: {
    target: 'baseline-widely-available',
    sourcemap: false,
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (/[\\/]node_modules[\\/](react|react-dom|scheduler)[\\/]/.test(id)) {
              return 'vendor-react';
            }
            if (/[\\/]node_modules[\\/]@tanstack[\\/]/.test(id)) {
              return 'vendor-tanstack';
            }
            if (
              /[\\/]node_modules[\\/](@radix-ui|lucide-react|sonner)[\\/]/.test(id)
            ) {
              return 'vendor-ui';
            }
            if (
              /[\\/]node_modules[\\/](react-markdown|shiki|remark-gfm|micromark)[\\/]/.test(id)
            ) {
              return 'vendor-markdown';
            }
          }
        },
      },
    },
  },
  optimizeDeps: {
    include: ['react', 'react-dom', '@tanstack/react-query', '@tanstack/react-virtual', 'zustand', 'zod'],
  },
  server: {
    port: 5173,
    strictPort: true,
    host: '127.0.0.1',
    hmr: { overlay: true },
    watch: {
      ignored: ['**/target/**', '**/.git/**', '**/crates/**', '**/docker/**'],
    },
  },
});
