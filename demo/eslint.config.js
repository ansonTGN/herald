/**
 * ESLint Configuration for Herald Demo Tests
 *
 * Enforces Playwright best practices and prevents common anti-patterns.
 */

import eslint from '@eslint/js'
import tsParser from '@typescript-eslint/parser'
import tsPlugin from '@typescript-eslint/eslint-plugin'

export default [
  eslint.configs.recommended,
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
        project: './tsconfig.json',
      },
      globals: {
        process: 'readonly',
        console: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': tsPlugin,
    },
    rules: {
      // TypeScript specific rules
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',

      // P0: Prevent fixed delays (violates Playwright best practices)
      'no-restricted-syntax': [
        'warn',
        {
          selector: 'CallExpression[callee.object.name="page"][callee.property.name="waitForTimeout"]',
          message: [
            '⛔ Using page.waitForTimeout() is prohibited (P0 violation).',
            '',
            'Reason: Fixed delays make tests unreliable and slow.',
            '',
            'Alternatives:',
            '  - Use expect().toBeVisible() for element visibility',
            '  - Use waitForLoadState() for page load states',
            '  - Use waitForResponse() for API calls',
            '  - Use waitForURL() for navigation changes',
            '',
            'See: ../spec/demo/e2e-testing.md#延迟使用规范-p0---严重',
          ].join('\n'),
        },
      ],

      // General code quality rules
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-empty': 'warn',
      'no-empty-pattern': 'warn',
      'no-useless-escape': 'warn',
      'no-undef': 'off',

      // Require meaningful test.step descriptions
      // Note: This is a comment recommendation since test.step is Playwright-specific

      // Encourage const over let (helps prevent accidental reassignment)
      'prefer-const': 'error',

      // No unused variables (helps catch mistakes) - use TypeScript version instead
      'no-unused-vars': 'off',
    },
  },
  {
    // Ignore patterns
    ignores: [
      'node_modules/**',
      'test-results/**',
      'playwright-report/**',
      'playwright/.cache/**',
      'playwright.config.ts',
      'playwright-showcase-reporter.ts',
    ],
  },
]
