/**
 * Selector Validation Tool for Demo Diagnostics
 *
 * Purpose: Automatically classify selector issues to distinguish between:
 * - SPELLING_ERROR: Test selector has a typo (e.g., "submit-btn" vs "submit-button")
 * - MISSING_TESTID: Frontend is missing the data-testid attribute
 * - FRONTEND_RENDER: Selector exists but element has rendering/visibility issues
 * - OK: Selector is valid
 *
 * Usage in demo-diagnose:
 * ```typescript
 * import { validateSelector } from '../helpers/selector-validator'
 * const result = await validateSelector('[data-testid="submit-btn"]')
 * console.log(result.recommendation) // 'SPELLING_ERROR'
 * ```
 */

import { readFileSync, existsSync } from 'fs'
import { join } from 'path'

/**
 * Levenshtein distance calculation for string similarity
 *
 * Returns a number between 0 (no similarity) and 1 (identical)
 */
export function calculateSimilarity(str1: string, str2: string): number {
  const len1 = str1.length
  const len2 = str2.length

  // Empty string cases
  if (len1 === 0) return len2 === 0 ? 1 : 0
  if (len2 === 0) return 0

  // Create distance matrix
  const matrix: number[][] = []
  for (let i = 0; i <= len1; i++) {
    matrix[i] = [i]
  }
  for (let j = 0; j <= len2; j++) {
    matrix[0][j] = j
  }

  // Fill the matrix
  for (let i = 1; i <= len1; i++) {
    for (let j = 1; j <= len2; j++) {
      const cost = str1[i - 1] === str2[j - 1] ? 0 : 1
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1, // deletion
        matrix[i][j - 1] + 1, // insertion
        matrix[i - 1][j - 1] + cost // substitution
      )
    }
  }

  const distance = matrix[len1][len2]
  const maxLen = Math.max(len1, len2)

  // Convert distance to similarity score (0-1)
  return 1 - distance / maxLen
}

/**
 * Extract testid value from selector string
 *
 * Examples:
 * - '[data-testid="submit-btn"]' -> 'submit-btn'
 * - 'data-testid=submit-btn' -> 'submit-btn'
 * - '.some-class' -> null
 */
export function extractTestid(selector: string): string | null {
  // Match [data-testid="..."]
  const attrMatch = selector.match(/data-testid\s*=\s*["']([^"']+)["']/)
  if (attrMatch) {
    return attrMatch[1]
  }

  // Match [data-testid="..."] with brackets
  const bracketMatch = selector.match(/\[data-testid\s*=\s*["']([^"']+)["']\]/)
  if (bracketMatch) {
    return bracketMatch[1]
  }

  return null
}

/**
 * Find all similar selectors in the frontend code
 *
 * Searches for data-testid attributes that are similar to the target
 * Returns sorted list by similarity score (descending)
 */
export function findSimilarSelectors(
  targetTestid: string,
  frontendTestids: string[],
  threshold: number = 0.7
): Array<{ testid: string; similarity: number }> {
  const results = frontendTestids
    .map((testid) => ({
      testid,
      similarity: calculateSimilarity(targetTestid, testid),
    }))
    .filter((item) => item.similarity >= threshold)
    .sort((a, b) => b.similarity - a.similarity)

  return results
}

/**
 * Extract all data-testid values from selectors.ts
 */
export function extractSelectorsFromTs(): string[] {
  const selectorsPath = join(__dirname, '../selectors.ts')

  if (!existsSync(selectorsPath)) {
    return []
  }

  const content = readFileSync(selectorsPath, 'utf-8')

  // Extract all data-testid values from selectors.ts
  const matches = content.matchAll(/data-testid\s*=\s*["']([^"']+)["']/g)
  return Array.from(matches).map((match) => match[1])
}

/**
 * Extract all data-testid values from frontend source files
 *
 * Searches in frontend/src/ directory for data-testid attributes
 */
export function extractFrontendTestids(): string[] {
  const frontendPath = join(process.cwd(), '../frontend/src')

  if (!existsSync(frontendPath)) {
    return []
  }

  try {
    const { readdirSync, readFileSync } = require('fs')
    const { join: pathJoin } = require('path')

    const testids = new Set<string>()

    // Recursive scan for .tsx and .ts files
    function scanDirectory(dir: string) {
      try {
        const entries = readdirSync(dir, { withFileTypes: true })

        for (const entry of entries) {
          const fullPath = pathJoin(dir, entry.name)

          if (entry.isDirectory()) {
            // Skip node_modules and build directories
            if (!entry.name.startsWith('.') && entry.name !== 'node_modules') {
              scanDirectory(fullPath)
            }
          } else if (entry.isFile() && (entry.name.endsWith('.tsx') || entry.name.endsWith('.ts'))) {
            // Extract data-testid values from the file
            const content = readFileSync(fullPath, 'utf-8')
            const matches = content.matchAll(/data-testid\s*=\s*["']([^"']+)["']/g)

            for (const match of matches) {
              testids.add(match[1])
            }
          }
        }
      } catch (error) {
        // Ignore errors (permission denied, etc.)
      }
    }

    scanDirectory(frontendPath)

    return Array.from(testids)
  } catch (error) {
    console.error('[SelectorValidator] Failed to scan frontend testids:', error)
    return []
  }
}

/**
 * Check if a testid exists in the frontend code
 *
 * @param testid - The data-testid value to search for
 * @returns true if found in frontend, false otherwise
 */
export function checkFrontendCode(testid: string): boolean {
  // This is a simplified check
  // Full implementation would:
  // 1. Search frontend/src/ for data-testid="${testid}"
  // 2. Return true if found, false otherwise

  const frontendTestids = extractFrontendTestids()
  return frontendTestids.includes(testid)
}

/**
 * Validation result types
 */
export type RecommendationType =
  | 'SPELLING_ERROR'
  | 'MISSING_TESTID'
  | 'FRONTEND_RENDER'
  | 'OK'

/**
 * Selector validation result
 */
export interface ValidationResult {
  testSelector: string
  testid: string | null
  inSelectorsTs: boolean
  inFrontend: boolean
  similarSelectors: string[]
  similarity: number
  recommendation: RecommendationType
  suggestedFix?: string
}

/**
 * Main validation function
 *
 * Validates a test selector and provides diagnostic recommendation
 *
 * @param testSelector - The selector string from the test (e.g., '[data-testid="submit-btn"]')
 * @param options - Optional configuration
 * @returns ValidationResult with recommendation
 */
export async function validateSelector(
  testSelector: string,
  options: {
    similarityThreshold?: number
    spellingThreshold?: number
  } = {}
): Promise<ValidationResult> {
  const { similarityThreshold = 0.7, spellingThreshold = 0.8 } = options

  // Extract testid value
  const testid = extractTestid(testSelector)

  if (!testid) {
    // Not a data-testid selector
    return {
      testSelector,
      testid: null,
      inSelectorsTs: false,
      inFrontend: false,
      similarSelectors: [],
      similarity: 0,
      recommendation: 'OK', // Not a data-testid selector, assume OK
    }
  }

  // Check in selectors.ts
  const selectorsTestids = extractSelectorsFromTs()
  const inSelectorsTs = selectorsTestids.includes(testid)

  // Check in frontend code
  const inFrontend = checkFrontendCode(testid)

  // Find similar selectors in frontend
  const frontendTestids = extractFrontendTestids()
  const similarSelectors = findSimilarSelectors(
    testid,
    frontendTestids,
    similarityThreshold
  )

  const topSimilarity =
    similarSelectors.length > 0 ? similarSelectors[0].similarity : 0

  // Determine recommendation
  let recommendation: RecommendationType
  let suggestedFix: string | undefined

  if (topSimilarity >= spellingThreshold && !inFrontend) {
    // High similarity + not in frontend = likely spelling error
    recommendation = 'SPELLING_ERROR'
    suggestedFix = similarSelectors[0].testid
  } else if (!inFrontend && similarSelectors.length === 0) {
    // Not in frontend + no similar selectors = missing testid
    recommendation = 'MISSING_TESTID'
  } else if (inFrontend && !inSelectorsTs) {
    // In frontend but not in selectors.ts = might need to add to selectors
    recommendation = 'FRONTEND_RENDER'
  } else {
    // Selector exists and is valid
    recommendation = 'OK'
  }

  return {
    testSelector,
    testid,
    inSelectorsTs,
    inFrontend,
    similarSelectors: similarSelectors.map((s) => s.testid),
    similarity: topSimilarity,
    recommendation,
    suggestedFix,
  }
}

/**
 * Batch validate multiple selectors
 *
 * Useful for validating all selectors in a test file
 */
export async function validateSelectors(
  selectors: string[],
  options?: Parameters<typeof validateSelector>[1]
): Promise<ValidationResult[]> {
  const results = await Promise.all(
    selectors.map((selector) => validateSelector(selector, options))
  )
  return results
}

/**
 * Generate diagnostic message from validation result
 *
 * Returns a human-readable message for the diagnostic report
 */
export function generateDiagnosticMessage(
  result: ValidationResult
): string {
  const { testSelector, testid, recommendation, similarSelectors, similarity, suggestedFix } = result

  switch (recommendation) {
    case 'SPELLING_ERROR':
      return `Selector spelling error detected:
- Test selector: ${testSelector}
- Extracted testid: "${testid}"
- Similar selector found: "${suggestedFix}" (similarity: ${(similarity * 100).toFixed(1)}%)
- Recommendation: Correct test selector to use "${suggestedFix}"
- Fix: demo-dev should update the test selector`

    case 'MISSING_TESTID':
      return `Frontend missing data-testid:
- Test selector: ${testSelector}
- Extracted testid: "${testid}"
- Frontend status: Not found in frontend code
- Recommendation: Add data-testid="${testid}" to the frontend component
- Fix: frontend-dev should add the missing data-testid attribute`

    case 'FRONTEND_RENDER':
      return `Selector exists but may have rendering issues:
- Test selector: ${testSelector}
- Extracted testid: "${testid}"
- Frontend status: Found in frontend code
- Recommendation: Check element visibility, rendering, or accessibility
- Fix: Continue with Step 2.7 (Deep Element Accessibility Check)`

    case 'OK':
      return `Selector validation passed:
- Test selector: ${testSelector}
- Extracted testid: "${testid}"
- Validation: Selector is correct and exists in frontend
- Recommendation: Continue with other diagnostic checks`

    default:
      return `Unknown validation result for selector: ${testSelector}`
  }
}
