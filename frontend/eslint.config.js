import js from '@eslint/js'
import tseslint from '@typescript-eslint/eslint-plugin'
import tsparser from '@typescript-eslint/parser'
import react from 'eslint-plugin-react'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

export default [
  // 忽略文件
  {
    ignores: [
      'dist',
      'build',
      'node_modules',
      'api.json',
      'playwright-report',
      'src/api/**', // 自动生成的 API 代码
      'src/lib/api-generated/**', // 自动生成的 API 客户端代码
    ],
  },
  // 基础 JS 配置
  js.configs.recommended,
  // TypeScript 配置
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
      globals: {
        browser: true,
        es2021: true,
        node: true,
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
      react,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      ...react.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      // React 设置
      'react/react-in-jsx-scope': 'off', // React 19 不需要
      'react/prop-types': 'off', // 使用 TypeScript
      'react/no-children-prop': 'off', // TanStack Form 使用 children 作为 render prop
      'react/no-unescaped-entities': 'off', // 允许直接使用撇号等字符
      'no-undef': 'off', // TypeScript 已处理，不需要此规则
      // TypeScript 规则调整
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
  {
    files: ['**/__tests__/**/*.{ts,tsx}', 'src/test/**/*.{ts,tsx}'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      'react-refresh/only-export-components': 'off',
    },
  },
  // TanStack Form 的类型系统限制 - 需要在某些地方使用 any 来避免复杂的泛型
  {
    files: [
      'src/components/billing/plan-form-basic-fields.tsx',
      'src/components/billing/plan-form-advanced-fields.tsx',
      'src/components/billing/plan-form-pricing-fields.tsx',
      'src/components/billing/plan-form-provider-fields.tsx',
      'src/components/shared/create-resource-dialog.tsx',
      'src/components/shared/edit-resource-dialog.tsx',
      'src/components/ui/tanstack-form.tsx',
    ],
    rules: {
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
  // TanStack Table 的 useReactTable API 返回的函数无法被安全 memoize，这是库的已知限制
  {
    files: ['src/components/billing/plan-table.tsx'],
    rules: {
      'react-hooks/incompatible-library': 'off',
    },
  },
  // TanStack Form wizard context intentionally keeps a stable Provider value
  // while updating the contained form callbacks to avoid an infinite render loop.
  {
    files: ['src/components/client-apps/wizard-form-context.tsx'],
    rules: {
      'react-hooks/refs': 'off',
    },
  },
]
