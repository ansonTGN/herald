import { defineConfig, devices } from '@playwright/test'
// import ShowcaseReporter from './playwright-showcase-reporter' // Unused import

/**
 * Playwright Demo Test Configuration
 *
 * 测试策略：以 Demo 为主，支持单模式运行
 *
 * Demo 测试用途：
 * - 完整用户故事覆盖（基于 docs/user-stories/*.md）
 * - 端到端业务流程
 * - 产品展示和用户培训
 *
 * 单一模式配置：
 * - demo-fast: 快速模式（headless），用于开发和验证
 */
export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.e2e.ts',

  // Optimized timeout configuration for comprehensive scenarios
  timeout: 120 * 1000, // 120 seconds (单个测试总时长) - increased for comprehensive scenarios
  expect: {
    timeout: 10 * 1000, // 10 seconds (断言超时) - increased for complex operations
  },

  retries: 0, // No retries for demos
  fullyParallel: false,
  workers: 1, // Single worker for demo

  outputDir: 'test-results/artifacts',

  use: {
    baseURL: 'http://localhost:3000',
    screenshot: 'off', // Disabled - screenshots not needed for demos
    video: 'off', // Disabled - video recording not needed for demos
    trace: 'off', // Disabled - trace not needed for demos (can be enabled with DEBUG=pw:api for debugging)
    // 使用 Playwright 的自动等待机制，减少超时配置
    actionTimeout: 0, // 无超时，使用自动等待（Playwright 最佳实践）
    navigationTimeout: 15 * 1000, // 15 seconds (导航超时) - reduced from 30s
  },

  projects: [
    // 快速模式（headless，用于开发和验证）
    {
      name: 'demo-fast',
      maxFailures: 1, // Stop on first failure
      use: {
        ...devices['Desktop Chrome'],
        headless: true,
        launchOptions: {
          args: [
            '--lang=en-US',
            '--enable-logging',
            '--log-level=0',
            '--disable-features=TranslateUI',
            '--no-first-run',
            '--no-default-browser-check',
          ],
        },
      },
    },
  ],

  reporter: [
    ['html', { open: 'never' }],
    ['list'],  // Enable list reporter for real-time test progress
    // [ShowcaseReporter], // Temporarily disabled
  ],

  // Quiet mode: Suppress Playwright's own output to reduce token usage
  quiet: false, // Can be overridden with --quiet flag
})

