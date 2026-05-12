import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

/**
 * Vitest 配置 - Node 环境
 *
 * 用于运行不需要浏览器环境的单元测试，例如 API 测试
 */
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    environment: 'node', // 使用 Node.js 环境
    include: ['**/__tests__/**/*.test.ts'],
    // 排除需要浏览器环境的测试
    exclude: ['**/__tests__/**/*.test.tsx', '**/node_modules/**'],
    globals: true,
  },
})
