import { defineConfig } from '@hey-api/openapi-ts'

export default defineConfig({
  input: './api.json',
  output: {
    // 输出目录
    path: './src/lib/api-generated',
  },
  services: {
    asClass: false, // 使用函数式调用（更简洁）
    name: '{{name}}',
    include: 'responses|requests|all',
    operationId: true,
    response: 'body',
  },
  client: 'axios',
})
