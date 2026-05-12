import { beforeAll, afterAll } from 'vitest'

beforeAll(async () => {
  // 全局测试钩子
  console.log('Starting E2E tests...')
})

afterAll(async () => {
  // 清理操作
  console.log('E2E tests completed')
})
