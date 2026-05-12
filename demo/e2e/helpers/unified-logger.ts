/**
 * Unified Logger for Playwright Demo Tests
 *
 * 整合 NetworkLogger、ConsoleLogger 和 RouteLogger，提供统一的日志记录接口
 * 所有日志自动写入文件，便于 AI 分析问题
 */

import { Page } from '@playwright/test'
import * as path from 'path'
import * as fs from 'fs'
import { NetworkLogger } from './network-logger'
import { ConsoleLogger } from './console-logger'
import { RouteLogger } from './route-logger'
import { TestCodeLogger } from './test-code-logger'
import { getLoggerConfig, LoggerConfig } from './log-config'

export interface TestLogs {
  networkLogs: string
  consoleLogs: string
  timestamp: string
}

export class UnifiedLogger {
  public network: NetworkLogger
  public console: ConsoleLogger
  public route: RouteLogger
  public testCode: TestCodeLogger
  private quietMode: boolean
  private compactMode: boolean
  private config: LoggerConfig

  constructor(page: Page, testTitle: string, quietMode?: boolean) {
    this.config = getLoggerConfig()
    this.quietMode = quietMode ?? this.config.quietMode
    this.compactMode = this.config.compactMode

    this.network = new NetworkLogger(page, this.quietMode)
    this.console = new ConsoleLogger(page, testTitle, this.quietMode, this.config.deduplicationEnabled, true, this.compactMode)
    this.route = new RouteLogger(page, this.quietMode)
    this.testCode = new TestCodeLogger(this.console.logFile, this.config.testCodeLoggingEnabled)
  }

  async finalize(): Promise<TestLogs> {
    // Restore original console methods (must be done before console.finalize())
    this.testCode.restoreConsole()

    // 完成控制台日志
    await this.console.finalize()

    const baseName = this.console.logFile.replace(/\.log$/, '')

    // 保存完整的网络日志到文件（包含所有请求，可通过 Request ID 快速定位）
    const networkLogFile = `${baseName}-network.json`
    const networkLogsData = this.network.exportToJson()
    fs.writeFileSync(networkLogFile, networkLogsData)

    return {
      networkLogs: networkLogFile,
      consoleLogs: this.console.logFile,
      timestamp: new Date().toISOString()
    }
  }

  printSummary(title: string) {
    if (this.quietMode) {
      this.printMiniSummary(title)
      return
    }

    this.printVerboseSummary(title)
  }

  private printVerboseSummary(title: string): void {
    console.log(`\n=== ${title} ===`)
    this.network.printLogs(title)
    this.route.printRouteChanges(title)
    console.log(`\nConsole logs: ${this.console.logFile}`)
    console.log(`\n[Summary]`)

    const logs = this.network.getLogs()
    const failedLogs = this.network.getFailedLogs()
    const routeAnalysis = this.route.analyzeRoutePattern()
    const consoleLogs = this.console.getLogs()
    const testCodeLogs = this.testCode.getLogs()

    console.log(`  Network requests: ${logs.length}`)
    console.log(`  Failed requests: ${failedLogs.length}`)
    console.log(`  Route changes: ${routeAnalysis.totalChanges}`)
    console.log(`  Possible redirects: ${routeAnalysis.possibleRedirects}`)
    console.log(`  Console logs: ${consoleLogs.length}`)
    console.log(`  Console errors: ${consoleLogs.filter(l => l.type === 'error').length}`)
    console.log(`  Console warnings: ${consoleLogs.filter(l => l.type === 'warning').length}`)
    console.log(`  Test code logs: ${testCodeLogs.length}`)
    console.log(`  Test code errors: ${this.testCode.getErrorLogs().length}`)
    console.log(`  Test code warnings: ${this.testCode.getWarningLogs().length}`)
  }

  /**
   * Mini summary for quiet mode - Single line output to reduce token usage
   */
  printMiniSummary(title?: string) {
    const logs = this.network.getLogs()
    const failedLogs = this.network.getFailedLogs()
    const consoleLogs = this.console.getLogs()
    const routeAnalysis = this.route.analyzeRoutePattern()
    const testCodeLogs = this.testCode.getLogs()
    const errors = consoleLogs.filter(l => l.type === 'error').length
    const warnings = consoleLogs.filter(l => l.type === 'warning').length

    if (this.compactMode) {
      console.log(`[${title || 'Uni'}] ${logs.length}r/${failedLogs.length}f ${errors}e/${warnings}w ${routeAnalysis.totalChanges}rt/${testCodeLogs.length}tl`)
      return
    }

    const relativePath = path.relative(process.cwd(), this.console.logFile)
    console.log(`[Uni] ${logs.length}r/${failedLogs.length}f ${errors}e/${warnings}w ${routeAnalysis.totalChanges}rt/${testCodeLogs.length}tl → ${relativePath}`)
  }

  printFailedLogs(title: string) {
    this.network.printFailedLogs(title)
  }

  printRouteChanges(title?: string) {
    this.route.printRouteChanges(title)
  }
}

/**
 * Test fixture with unified logger (currently unused)
 *
 * ✅ 修复：正确获取 testInfo
 * Playwright fixture 的 use 函数不接受参数，testInfo 需要通过其他方式获取
 *
 * @deprecated - Not currently used, commented out to avoid type errors
 */
/*
export const testWithUnifiedLogger = (
  base: import('@playwright/test').TestType<{}, import('@playwright/test').WorkerInfo>
) =>
  base.extend<{
  logger: UnifiedLogger
}>({
  logger: [async ({ page }, use) => {
    const testTitle = 'test' // Will be set by test code
    const logger = new UnifiedLogger(page, testTitle)
    await use(logger)
  }, { scope: 'test' }],
})
*/
