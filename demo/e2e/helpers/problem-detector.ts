/**
 * Common Problem Auto-Detector for Demo Tests
 *
 * Automatically detects common problems based on error patterns
 * Provides pre-defined fix suggestions from the mistake database
 *
 * Usage in demo-diagnose:
 * ```typescript
 * import { detectProblem } from '../helpers/problem-detector'
 * const problem = detectProblem(errorMessage)
 * if (problem) {
 *   console.log(problem.fix)
 * }
 * ```
 */

import { readFileSync, existsSync } from 'fs'
import { join } from 'path'

/**
 * Detected problem with fix suggestion
 */
export interface DetectedProblem {
  pattern: string | RegExp
  severity: 'P0' | 'P1' | 'P2' | 'P3'
  category: string
  fix: string
  successRate?: number
  reference?: string // Reference to mistake database entry
  codeExamples?: {
    bad: string
    good: string
  }
}

/**
 * Pattern match result with confidence
 */
export interface PatternMatch {
  problem: DetectedProblem
  confidence: number // 0-1
  matchedText: string
}

/**
 * Common problem patterns from mistake database
 */
const COMMON_PROBLEMS: DetectedProblem[] = [
  {
    pattern: /toast.*not.*found|expect.*toast/i,
    severity: 'P0',
    category: 'TEST_VALIDATION',
    fix: 'Do NOT use toast as validation condition. Verify actual business results: list update, page navigation, dialog closure, data existence. Toast is unstable (delayed appearance, auto-disappear, i18n variations). See .claude/mistakes/demo-repair.md section: "使用 toast 作为验证条件"',
    successRate: 95,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `await page.click('[data-testid="submit-button"]')
await expect(page.locator('[data-testid="toast"]')).toBeVisible()
await expect(page.locator('[data-testid="toast"]')).toContainText('Success')`,
      good: `await page.click('[data-testid="submit-button"]')
await expect(page.locator('[data-testid="item-list"]')).toContainText('New Item')
// OR
await expect(page).toHaveURL('/items/new-item')
// OR
await expect(page.locator('[role="dialog"]')).toBeHidden()`,
    },
  },
  {
    pattern: /login.*form.*not.*found|waiting.*login.*form/i,
    severity: 'P0',
    category: 'AUTH_STATE',
    fix: 'User is already logged in and was redirected to dashboard. LoginPage.loginAsAdmin() should check current URL and skip login if already authenticated. See .claude/mistakes/demo-repair.md section: "登录状态冲突"',
    successRate: 90,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `async loginAsAdmin(email: string, password: string) {
  // Always tries to login, even if already authenticated
  await this.goto()
  await this.fillForm(email, password)
  await this.submit()
}`,
      good: `async loginAsAdmin(email: string, password: string, realmId: string) {
  const currentUrl = this.page.url()

  // Check if already on dashboard (authenticated)
  if (currentUrl.includes('/dashboard')) {
    console.log('Already logged in, skipping login')
    return
  }

  // Check cookies for auth token
  const cookies = await this.page.context().cookies()
  const xAuthCookie = cookies.find(c => c.name === 'X-Auth')

  const isAdminPage = new RegExp(\`^http://localhost:3000/\${realmId}(/|$)\`).test(currentUrl)

  if (xAuthCookie && isAdminPage) {
    console.log('Already logged in, skipping login')
    return
  }

  // Perform login
  await this.goto(realmId)
  await this.login({ email, password })
}`,
    },
  },
  {
    pattern: /strict.*mode.*violation|locator.*strict.*mode/i,
    severity: 'P1',
    category: 'SELECTOR_STRICTNESS',
    fix: 'Use .first() or more specific selectors to handle strict mode violations. Playwright strict mode requires exactly one match for locator actions.',
    successRate: 95,
    codeExamples: {
      bad: `// Multiple elements match, but strict mode expects exactly one
page.click('[data-testid="button"]')`,
      good: `// Use .first() to get first matching element
page.locator('[data-testid="button"]').first().click()

// OR use more specific selector
page.click('[data-testid="submit-button"]')`,
    },
  },
  {
    pattern: /waiting.*locator.*data-testid=.*-input|input.*not.*found/i,
    severity: 'P0',
    category: 'MISSING_TESTID',
    fix: 'Frontend component is missing data-testid attribute. Add data-testid="xxx" to the input element. See .claude/mistakes/demo-repair.md section: "缺少 data-testid 属性"',
    successRate: 95,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `<input type="email" name="email" className="form-control" />
<!-- Test expects: [data-testid="email-input"] -->`,
      good: `<input data-testid="email-input" type="email" name="email" className="form-control" />
<!-- Test selector: [data-testid="email-input"] -->`,
    },
  },
  {
    pattern: /401.*unauthorized|invalid.*credentials/i,
    severity: 'P0',
    category: 'AUTH_FAILURE',
    fix: 'Login API returned 401. Check if frontend is sending correct field (email vs username). Backend expects "email" field for email-based login. See .claude/mistakes/demo-repair.md section: "前端登录使用错误字段"',
    successRate: 95,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `// Frontend sends "username" field
const loginData = {
  username: 'user@example.com',
  password: 'password',
}`,
      good: `// Frontend intelligently chooses "email" or "username"
const isEmail = values.username.includes('@')
const loginData = {
  email: isEmail ? values.username : undefined,
  username: isEmail ? undefined : values.username,
  password: values.password,
}`,
    },
  },
  {
    pattern: /data-testid=["']([^"']+)["']\s+.*["']([^"']+)["']/,
    severity: 'P0',
    category: 'DATA_TESTID_ALIAS',
    fix: `Using space-separated values in data-testid does NOT create multiple selectors. CSS selectors expect exact match. Use unique value or multiple data-* attributes. See .claude/mistakes/demo-repair.md section: "使用空格分隔的 data-testid 值作为 '别名'"`,
    successRate: 99,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `<Input data-testid="username-input email-input" />
<!-- Test expects: [data-testid="email-input"] -->
<!-- CSS selector [data-testid="email-input"] will NOT match "username-input email-input" -->`,
      good: `<Input data-testid="email-input" />
<!-- OR -->
<Input data-testid="email-input" data-legacy="username-input" />
<!-- OR -->
<Input data-testid="email-input" className="username-field" />`,
    },
  },
  {
    pattern: /button.*has-text.*[\"']create[\"'].*not.*found|button.*has-text.*[\"']add[\"'].*not.*found/i,
    severity: 'P0',
    category: 'BUTTON_TEXT_MISMATCH',
    fix: 'Button text mismatch between frontend and test. Standardize button text (prefer "Add" over "Create"). See .claude/mistakes/demo-repair.md section: "按钮文本不匹配 ("Create" vs "Add")"',
    successRate: 90,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `// Frontend: "Create User"
// Test expects: "Add User"
await page.click('button:has-text("Add User")')`,
      good: `// Frontend: "Add User"
// Test expects: "Add User"
await page.click('button:has-text("Add User")')

// OR use role-based selector (recommended)
await page.getByRole('button', { name: 'Add User' }).click()`,
    },
  },
  {
    pattern: /css.*selector|\.btn-primary|\.mui-|class=.*button/i,
    severity: 'P0',
    category: 'CSS_SELECTOR',
    fix: 'Do NOT use CSS class selectors. They are unstable and change with UI framework updates. Use semantic selectors (getByRole, getByLabel) or data-testid. See .claude/mistakes/demo-repair.md section: "使用 CSS 类选择器"',
    successRate: 90,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `await page.click('.btn-primary')
await page.click('.MuiButton-root')
await page.locator('.submit-button').click()`,
      good: `// Use semantic selectors (recommended)
await page.getByRole('button', { name: 'Submit' }).click()

// OR use data-testid
await page.click('[data-testid="submit-button"]')`,
    },
  },
  {
    pattern: /waitForTimeout|delay|sleep/i,
    severity: 'P1',
    category: 'FIXED_DELAY',
    fix: 'Avoid using fixed delays (page.waitForTimeout, demoDelay). Use Playwright auto-wait with expect().toBeVisible(). Fixed delays cause slow and unstable tests. See .claude/mistakes/demo-repair.md section: "过度使用固定延迟（Fixed Delays）"',
    successRate: 95,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `await page.click(button)
await page.waitForTimeout(1500)  // Fixed delay

await demoDelay(page, 1500)  // Demo delay (no-op in current implementation)`,
      good: `await page.click(button)
// Use auto-wait with assertion
await expect(page.locator(result)).toBeVisible()

// Configure reasonable timeouts in playwright.config.ts
// timeout: 30 * 1000,  // 30 seconds (single test total)
// expect: { timeout: 5 * 1000 },  // 5 seconds (assertion timeout)`,
    },
  },
  {
    pattern: /user-table|users-table.*not.*found/i,
    severity: 'P0',
    category: 'NAMING_CONVENTION',
    fix: 'Data-testid naming inconsistency (singular vs plural). Standardize: use plural for tables/lists, singular for individual fields. See .claude/mistakes/demo-repair.md section: "data-testid 命名不一致"',
    successRate: 95,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `// Frontend: user-table (singular)
// Test expects: users-table (plural)
<Table data-testid="user-table">
await page.locator('[data-testid="users-table"]')`,
      good: `// Frontend: users-table (plural for table)
// Test expects: users-table (plural)
<Table data-testid="users-table">
<Table data-testid="roles-table">

// Singular for individual fields
<Input data-testid="email-input">
<Button data-testid="submit-button">`,
    },
  },
  {
    pattern: /expect.*is.*not.*defined.*demoLogger/i,
    severity: 'P0',
    category: 'API_MISUSE',
    fix: 'Do NOT call demoLogger.log() directly. Use console.log() instead. UnifiedLogger automatically intercepts console.log() calls. See .claude/mistakes/demo-repair.md section: "调用 demoLogger.log() API"',
    successRate: 100,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `import { demoLogger } from '../fixtures/demo-page.fixtures'
demoLogger.log('Creating user:', userData)  // ✗ TypeError: demoLogger.log is not a function`,
      good: `// Directly use console.log() - automatically intercepted by TestCodeLogger
console.log('Creating user:', userData)  // ✓`,
    },
  },
  {
    pattern: /expect.*is.*not.*defined/i,
    severity: 'P0',
    category: 'MISSING_IMPORT',
    fix: 'Missing "expect" import statement. Add "expect" to the import from demo-page.fixtures. See .claude/mistakes/demo-repair.md section: "缺少 expect 导入语句"',
    successRate: 100,
    reference: '.claude/mistakes/demo-repair.md',
    codeExamples: {
      bad: `import { test, cleanupTestData } from '../fixtures/demo-page.fixtures'
// Missing "expect" import
expect(page.locator('h1')).toBeVisible()  // ✗ ReferenceError: expect is not defined`,
      good: `import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
expect(page.locator('h1')).toBeVisible()  // ✓`,
    },
  },
]

/**
 * Detect common problem from error message
 *
 * @param errorMessage The error message from Playwright
 * @returns Detected problem or null if no pattern matches
 */
export function detectProblem(errorMessage: string): DetectedProblem | null {
  const errorLower = errorMessage.toLowerCase()

  for (const problem of COMMON_PROBLEMS) {
    if (problem.pattern instanceof RegExp) {
      const match = errorMessage.match(problem.pattern)
      if (match) {
        return problem
      }
    } else if (errorLower.includes(problem.pattern.toLowerCase())) {
      return problem
    }
  }

  return null
}

/**
 * Detect all matching problems (multiple patterns may match)
 *
 * @param errorMessage The error message from Playwright
 * @returns Array of detected problems sorted by severity
 */
export function detectProblems(errorMessage: string): DetectedProblem[] {
  const detected: DetectedProblem[] = []
  const errorLower = errorMessage.toLowerCase()

  for (const problem of COMMON_PROBLEMS) {
    if (problem.pattern instanceof RegExp) {
      const match = errorMessage.match(problem.pattern)
      if (match) {
        detected.push(problem)
      }
    } else if (errorLower.includes(problem.pattern.toLowerCase())) {
      detected.push(problem)
    }
  }

  // Sort by severity (P0 first, then P1, etc.)
  const severityOrder = { P0: 0, P1: 1, P2: 2, P3: 3 }
  detected.sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity])

  return detected
}

/**
 * Load custom problems from mistake database
 *
 * @param mistakeDbPath Path to mistake database file
 * @returns Array of custom problems
 */
export function loadProblemsFromMistakeDb(mistakeDbPath: string): DetectedProblem[] {
  if (!existsSync(mistakeDbPath)) {
    return []
  }

  try {
    const content = readFileSync(mistakeDbPath, 'utf-8')
    // This is a simplified parser - full implementation would parse the markdown format
    // and extract problems with patterns and fixes
    return []
  } catch (error) {
    console.error('[ProblemDetector] Failed to load mistake database:', error)
    return []
  }
}

/**
 * Get problem statistics for diagnostic report
 *
 * @param detectedProblems Array of detected problems
 * @returns Statistics summary
 */
export function getProblemStatistics(
  detectedProblems: DetectedProblem[]
): {
  count: number
  byCategory: Record<string, number>
  bySeverity: Record<string, number>
  highestSeverity: string
} {
  const stats = {
    count: detectedProblems.length,
    byCategory: {} as Record<string, number>,
    bySeverity: { P0: 0, P1: 0, P2: 0, P3: 0 },
    highestSeverity: 'P3',
  }

  for (const problem of detectedProblems) {
    // Count by category
    stats.byCategory[problem.category] = (stats.byCategory[problem.category] || 0) + 1

    // Count by severity
    stats.bySeverity[problem.severity]++
  }

  // Determine highest severity
  if (stats.bySeverity.P0 > 0) {
    stats.highestSeverity = 'P0'
  } else if (stats.bySeverity.P1 > 0) {
    stats.highestSeverity = 'P1'
  } else if (stats.bySeverity.P2 > 0) {
    stats.highestSeverity = 'P2'
  } else {
    stats.highestSeverity = 'P3'
  }

  return stats
}

/**
 * Format problem detection result for diagnostic report
 *
 * @param detectedProblem Detected problem
 * @param errorMessage Original error message
 * @returns Formatted markdown string
 */
export function formatDetectedProblem(
  detectedProblem: DetectedProblem,
  errorMessage: string
): string {
  const lines: string[] = []

  lines.push('## 检测到常见问题')
  lines.push('')
  lines.push(`**严重级别**: ${detectedProblem.severity}`)
  lines.push(`**问题分类**: ${detectedProblem.category}`)
  lines.push('')
  lines.push('### 修复建议')
  lines.push('')
  lines.push(detectedProblem.fix)
  lines.push('')

  if (detectedProblem.reference) {
    lines.push(`**参考**: ${detectedProblem.reference}`)
    lines.push('')
  }

  if (detectedProblem.codeExamples) {
    lines.push('### 代码示例')
    lines.push('')
    lines.push('```typescript')
    lines.push('// ❌ 错误写法')
    lines.push(detectedProblem.codeExamples.bad)
    lines.push('')
    lines.push('// ✅ 正确写法')
    lines.push(detectedProblem.codeExamples.good)
    lines.push('```')
    lines.push('')
  }

  if (detectedProblem.successRate) {
    lines.push(`**成功率**: ${detectedProblem.successRate}%`)
    lines.push('')
  }

  return lines.join('\n')
}

/**
 * Check if error matches any known problem pattern
 *
 * @param errorMessage The error message
 * @returns true if a known problem pattern matches
 */
export function hasKnownProblem(errorMessage: string): boolean {
  return detectProblem(errorMessage) !== null
}

/**
 * Get the highest priority problem from error message
 *
 * @param errorMessage The error message
 * @returns Highest priority problem or null
 */
export function getHighestPriorityProblem(errorMessage: string): DetectedProblem | null {
  const problems = detectProblems(errorMessage)
  return problems.length > 0 ? problems[0] : null
}
