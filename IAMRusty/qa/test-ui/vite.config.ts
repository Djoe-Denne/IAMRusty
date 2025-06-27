import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  optimizeDeps: {
    include: ['lucide-react'],
  },
  server: {
    proxy: {
      '/api': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false // Allow self-signed certificates
      },
      '/internal': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false
      },
      '/.well-known': {
        target: 'https://localhost:8443',
        changeOrigin: true,
        secure: false
      }
    }
  }
});
