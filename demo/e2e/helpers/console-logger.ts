/**
 * Console Logger for Playwright Tests
 *
 * 捕获浏览器控制台日志并写入文件，供 AI 分析使用
 */

import { Page, ConsoleMessage } from '@playwright/test'
import * as fs from 'fs/promises'
import * as path from 'path'

export interface ConsoleLogEntry {
  timestamp: string
  type: string
  text: string
  location?: string
}

/**
 * Common harmless console messages that can be filtered out
 * to reduce noise in logs
 */
const FILTERED_PATTERNS = [
  // React DevTools
  /Download the React DevTools/i,
  /Install the React DevTools/i,

  // ReactDOM warnings (often harmless in development)
  /Warning: ReactDOM\.render/i,
  /Warning: react-dom/i,

  // Vite / HMR warnings
  /experimental/i,
  /HMR/i,

  // Source map warnings
  /SourceMap/i,
  /sourcemapped/i,

  // Third-party library warnings
  /Google Maps/i,
  /Recaptcha/i,
  /Stripe/i,

  // Network errors that are expected
  /net::ERR_BLOCKED_BY_CLIENT/i,  // Ad blocker
  /net::ERR_FAILED/i,  // Often cancelled requests
]

export class ConsoleLogger {
  private logs: ConsoleLogEntry[] = []
  private page: Page
  public logFile: string
  private quietMode: boolean
  private compactMode: boolean
  private deduplicationEnabled: boolean
  private filteringEnabled: boolean
  private seenLogs = new Set<string>()
  private filteredCount = 0

  constructor(
    page: Page,
    testTitle: string,
    quietMode = false,
    deduplicationEnabled = true,
    filteringEnabled = true,
    compactMode = false
  ) {
    this.page = page
    this.quietMode = quietMode
    this.compactMode = compactMode
    this.deduplicationEnabled = deduplicationEnabled
    this.filteringEnabled = filteringEnabled

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
    const safeTestTitle = testTitle.replace(/[^a-zA-Z0-9\-_]/g, '_')

    // Get RunId from environment variable (set by run-test-quiet.ps1)
    const runId = process.env.DEMO_RUN_ID || 'no-run-id'

    this.logFile = path.resolve(process.cwd(), 'test-results', 'unified-logs', `${runId}-${safeTestTitle}-${timestamp}.log`)

    this.attachListeners()
  }

  /**
   * Check if a log entry should be filtered out
   */
  private shouldFilter(entry: ConsoleLogEntry): boolean {
    if (!this.filteringEnabled || entry.type === 'error') {
      return false
    }

    return FILTERED_PATTERNS.some(pattern => pattern.test(entry.text))
  }

  /**
   * Check if a log entry is a duplicate
   */
  private isDuplicate(entry: ConsoleLogEntry): boolean {
    if (!this.deduplicationEnabled) {
      return false
    }

    const logKey = `${entry.type}:${entry.text}`
    if (this.seenLogs.has(logKey)) {
      return true
    }

    this.seenLogs.add(logKey)
    return false
  }

  /**
   * Add log entry with deduplication check
   */
  private addLog(entry: ConsoleLogEntry, skipDeduplication = false): boolean {
    if (skipDeduplication || !this.isDuplicate(entry)) {
      this.logs.push(entry)
      return true
    }
    return false
  }

  private attachListeners() {
    this.page.on('console', async (msg: ConsoleMessage) => {
      const entry: ConsoleLogEntry = {
        timestamp: new Date().toISOString(),
        type: msg.type(),
        text: msg.text(),
        location: msg.location()?.url,
      }

      // Apply intelligent filtering
      if (this.shouldFilter(entry)) {
        this.filteredCount++
        await this.writeLog(entry, false, true)
        return
      }

      const isDuplicate = !this.addLog(entry)
      await this.writeLog(entry, isDuplicate, false)
    })

    this.page.on('pageerror', async (error: Error) => {
      const entry: ConsoleLogEntry = {
        timestamp: new Date().toISOString(),
        type: 'error',
        text: `${error.message}\n${error.stack}`,
      }

      // Errors are never deduplicated or filtered
      this.addLog(entry, true)
      await this.writeLog(entry, false, false)
    })

    // Handle unhandled promise rejections
    this.page.on('pageerror', async (error: Error) => {
      if (!error.message.includes('UnhandledPromiseRejection')) {
        return
      }

      const entry: ConsoleLogEntry = {
        timestamp: new Date().toISOString(),
        type: 'warning',
        text: error.message,
      }

      const isDuplicate = !this.addLog(entry)
      await this.writeLog(entry, isDuplicate, false)
    })
  }

  private async writeLog(entry: ConsoleLogEntry, isDuplicate: boolean = false, isFiltered: boolean = false) {
    const dedupMarker = isDuplicate ? ' [DUPLICATE]' : ''
    const filterMarker = isFiltered ? ' [FILTERED]' : ''
    const logLine = `[${entry.timestamp}] [${entry.type.toUpperCase()}]${dedupMarker}${filterMarker} ${entry.text}${entry.location ? ` (${entry.location})` : ''}\n`

    try {
      // 确保目录存在
      await fs.mkdir(path.dirname(this.logFile), { recursive: true })
      // 追加写入文件
      await fs.appendFile(this.logFile, logLine)
    } catch (error) {
      // 如果文件写入失败，至少输出到控制台
      console.error('[ConsoleLogger] Failed to write log:', error)
    }
  }

  async finalize() {
    const dedupStats = this.getDeduplicationStats()
    const summary = {
      totalLogs: this.logs.length,
      uniqueLogs: this.logs.length,
      duplicatesFiltered: dedupStats.duplicatesFiltered,
      filteredCount: this.filteredCount,
      errors: this.logs.filter(l => l.type === 'error').length,
      warnings: this.logs.filter(l => l.type === 'warning').length,
      timestamp: new Date().toISOString(),
    }

    if (this.quietMode) {
      this.printQuietSummary(summary)
    } else {
      this.printVerboseSummary(summary)
    }
  }

  private printQuietSummary(summary: {
    errors: number
    warnings: number
  }): void {
    const dedupStats = this.getDeduplicationStats()
    const opt = [
      dedupStats.duplicatesFiltered && `${dedupStats.duplicatesFiltered}d`,
      this.filteredCount && `${this.filteredCount}f`
    ].filter(Boolean).join(',')

    if (this.compactMode) {
      console.log(`[Con] ${summary.errors}e/${summary.warnings}w${opt ? ` (${opt})` : ''}`)
      return
    }

    const relativePath = path.relative(process.cwd(), this.logFile)
    console.log(`[Con] ${summary.errors}e/${summary.warnings}w${opt ? ` (${opt})` : ''} → ${relativePath}`)
  }

  private printVerboseSummary(summary: {
    errors: number
    warnings: number
  }): void {
    const dedupStats = this.getDeduplicationStats()

    console.log(`[ConsoleLogger] Logs written to: ${this.logFile}`)
    console.log(`  Absolute path: ${path.resolve(this.logFile)}`)
    console.log(`[ConsoleLogger] Summary: ${summary.errors} errors, ${summary.warnings} warnings`)

    if (dedupStats.duplicatesFiltered > 0) {
      console.log(`[ConsoleLogger] Deduplication: ${dedupStats.duplicatesFiltered} duplicates filtered`)
    }
    if (this.filteredCount > 0) {
      console.log(`[ConsoleLogger] Filtering: ${this.filteredCount} messages filtered`)
    }
  }

  /**
   * Get deduplication statistics
   */
  getDeduplicationStats(): { duplicatesFiltered: number; uniqueLogs: number } {
    if (!this.deduplicationEnabled) {
      return { duplicatesFiltered: 0, uniqueLogs: this.logs.length }
    }

    return {
      duplicatesFiltered: this.seenLogs.size - this.logs.length,
      uniqueLogs: this.logs.length
    }
  }

  getLogs(): ConsoleLogEntry[] {
    return this.logs
  }

  clearLogs() {
    this.logs = []
  }
}
