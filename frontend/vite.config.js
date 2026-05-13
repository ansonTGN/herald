import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { tanstackRouter } from '@tanstack/router-plugin/vite'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    react(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  optimizeDeps: {
    include: ['@tanstack/react-query', '@tanstack/react-router'],
  },
  server: {
    host: true,
    allowedHosts: ['.ngrok-free.app'],
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        withCredentials: true, // 携带 cookies 以支持认证
      },
    },
  },
})
