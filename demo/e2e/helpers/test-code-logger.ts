/**
 * Test Code Logger
 *
 * Captures Node.js console output from Playwright test code.
 * Intercepts console.log/warn/error/info methods to capture output
 * while preserving original functionality.
 *
 * Log format: [timestamp] [TYPE] [TEST_CODE] message (test-file.ts:line)
 */

import * as fs from 'fs'
import * as path from 'path'

export interface TestCodeLogEntry {
  timestamp: string
  type: 'log' | 'info' | 'warn' | 'error'
  message: string
  sourceLocation?: string
}

type ConsoleMethod = 'log' | 'info' | 'warn' | 'error'

export class TestCodeLogger {
  private logs: TestCodeLogEntry[] = []
  private logFile: string
  private enabled: boolean
  private initialized = false
  private originalConsole: Map<ConsoleMethod, typeof console.log> = new Map()

  constructor(logFile: string, enabled: boolean = true) {
    this.logFile = logFile
    this.enabled = enabled

    // Auto-initialize if enabled
    if (this.enabled) {
      this.initialize()
    }
  }

  /**
   * Initialize console interception
   */
  private initialize(): void {
    if (this.initialized || !this.enabled) {
      return
    }

    const methods: ConsoleMethod[] = ['log', 'info', 'warn', 'error']

    for (const method of methods) {
      // Store original method
      this.originalConsole.set(method, console[method])

      // Intercept method
      console[method] = ((...args: unknown[]) => {
        this.captureLog(method, args)
        this.originalConsole.get(method)!.apply(console, args)
      }) as typeof console.log
    }

    this.initialized = true
  }

  /**
   * Capture console output with source location
   */
  private captureLog(type: 'log' | 'info' | 'warn' | 'error', args: unknown[]): void {
    if (!this.enabled) {
      return
    }

    const entry: TestCodeLogEntry = {
      timestamp: new Date().toISOString(),
      type,
      message: this.formatMessage(args),
      sourceLocation: this.extractSourceLocation(),
    }

    this.logs.push(entry)
    this.writeToFile(entry)
  }

  /**
   * Format console arguments into a string
   */
  private formatMessage(args: unknown[]): string {
    return args.map(formatArgument).join(' ')
  }

  /**
   * Extract source location from stack trace
   * Returns format: "test-file.ts:line" or undefined
   */
  private extractSourceLocation(): string | undefined {
    const stack = new Error().stack
    if (!stack) {
      return undefined
    }

    const lines = stack.split('\n')
    const patterns = [
      /\(([^:]+\.e2e\.ts):(\d+):\d+\)/,
      /\(([^:]+\.spec\.ts):(\d+):\d+\)/,
      /at\s+([^:]+\.e2e\.ts):(\d+)/,
      /at\s+([^:]+\.spec\.ts):(\d+)/,
    ]

    // Skip the first few lines (this function, captureLog, console method)
    for (let i = 4; i < lines.length; i++) {
      for (const pattern of patterns) {
        const match = lines[i].match(pattern)
        if (match) {
          return `${path.basename(match[1])}:${match[2]}`
        }
      }
    }

    return undefined
  }

  /**
   * Write a log entry to file
   */
  private writeToFile(entry: TestCodeLogEntry): void {
    try {
      const logLine = this.formatLogLine(entry)

      // Ensure directory exists
      const logDir = path.dirname(this.logFile)
      if (!fs.existsSync(logDir)) {
        fs.mkdirSync(logDir, { recursive: true })
      }

      // Append to file
      fs.appendFileSync(this.logFile, logLine + '\n')
    } catch (error) {
      // Fail silently to avoid infinite recursion
      // (calling console.error here would trigger interception again)
    }
  }

  /**
   * Format a log entry as a single line
   */
  private formatLogLine(entry: TestCodeLogEntry): string {
    const timestamp = entry.timestamp
    const type = entry.type.toUpperCase()
    const location = entry.sourceLocation ? ` (${entry.sourceLocation})` : ''

    return `[${timestamp}] [${type}] [TEST_CODE] ${entry.message}${location}`
  }

  /**
   * Restore original console methods
   */
  restoreConsole(): void {
    if (!this.initialized) {
      return
    }

    for (const [method, originalFn] of this.originalConsole.entries()) {
      console[method] = originalFn
    }

    this.initialized = false
  }

  /**
   * Get all captured logs
   */
  getLogs(): TestCodeLogEntry[] {
    return this.logs
  }

  /**
   * Get logs by type
   */
  getLogsByType(type: 'log' | 'info' | 'warn' | 'error'): TestCodeLogEntry[] {
    return this.logs.filter(log => log.type === type)
  }

  /**
   * Get error logs
   */
  getErrorLogs(): TestCodeLogEntry[] {
    return this.getLogsByType('error')
  }

  /**
   * Get warning logs
   */
  getWarningLogs(): TestCodeLogEntry[] {
    return this.getLogsByType('warn')
  }

  /**
   * Clear all logs (does not affect log file)
   */
  clearLogs(): void {
    this.logs = []
  }

  /**
   * Public log method - logs to console and captures output
   */
  log(...args: unknown[]): void {
    this.captureLog('log', args)
    this.originalConsole.get('log')!.apply(console, args)
  }

  /**
   * Public info method - logs to console and captures output
   */
  info(...args: unknown[]): void {
    this.captureLog('info', args)
    this.originalConsole.get('info')!.apply(console, args)
  }

  /**
   * Public warn method - logs to console and captures output
   */
  warn(...args: unknown[]): void {
    this.captureLog('warn', args)
    this.originalConsole.get('warn')!.apply(console, args)
  }

  /**
   * Public error method - logs to console and captures output
   */
  error(...args: unknown[]): void {
    this.captureLog('error', args)
    this.originalConsole.get('error')!.apply(console, args)
  }
}

/**
 * Format a single argument as a string
 */
function formatArgument(arg: unknown): string {
  if (typeof arg === 'string') {
    return arg
  }

  if (arg instanceof Error) {
    return `${arg.name}: ${arg.message}`
  }

  if (arg === null) {
    return 'null'
  }

  if (arg === undefined) {
    return 'undefined'
  }

  if (typeof arg === 'object') {
    try {
      return JSON.stringify(arg, null, 2)
    } catch {
      return String(arg)
    }
  }

  return String(arg)
}
