/**
 * Logger Configuration
 *
 * Environment variables for controlling demo test logging behavior
 */

import * as path from 'path'

export type LogLevel = 'mini' | 'normal' | 'verbose' | 'silent'

export interface LoggerConfig {
  quietMode: boolean
  deduplicationEnabled: boolean
  logLevel: LogLevel
  aggregateLogs: boolean
  testCodeLoggingEnabled: boolean
  compactMode: boolean
}

/**
 * Get logger configuration from environment variables
 *
 * Environment variables:
 * - DEMO_LOG_LEVEL: mini | normal | verbose | silent (default: mini)
 * - DEMO_LOG_DEDUP: true | false (default: true)
 * - DEMO_LOG_AGGREGATE: true | false (default: true)
 * - DEMO_LOG_TEST_CODE: true | false (default: true) - Enable test code logging
 */
export function getLoggerConfig(): LoggerConfig {
  const logLevel = (process.env.DEMO_LOG_LEVEL as LogLevel) || 'mini'
  const deduplicationEnabled = process.env.DEMO_LOG_DEDUP !== 'false'
  const aggregateLogs = process.env.DEMO_LOG_AGGREGATE !== 'false'
  const testCodeLoggingEnabled = process.env.DEMO_LOG_TEST_CODE !== 'false'

  // Determine quiet mode based on log level
  let quietMode = true
  if (logLevel === 'verbose') {
    quietMode = false
  } else if (logLevel === 'silent') {
    quietMode = true
  } else if (logLevel === 'normal') {
    quietMode = false
  } else {
    // 'mini' mode - quiet with minimal output
    quietMode = true
  }

  return {
    quietMode,
    deduplicationEnabled,
    logLevel,
    aggregateLogs,
    testCodeLoggingEnabled,
    compactMode: process.env.DEMO_LOG_COMPACT === 'true'
  }
}

/**
 * Check if we should suppress console output entirely
 */
export function isSilentMode(): boolean {
  return process.env.DEMO_LOG_LEVEL === 'silent'
}

/**
 * Check if we should show verbose output
 */
export function isVerboseMode(): boolean {
  return process.env.DEMO_LOG_LEVEL === 'verbose'
}

/**
 * Check if we should show mini summaries (default)
 */
export function isMiniMode(): boolean {
  return !process.env.DEMO_LOG_LEVEL || process.env.DEMO_LOG_LEVEL === 'mini'
}

/**
 * Get a display-friendly path for log files
 * Shows project context and absolute path
 */
export function formatLogPath(logFile: string): {
  context: string
  absolute: string
  display: string
} {
  const absolutePath = path.resolve(logFile)
  const relativePath = path.relative(process.cwd(), logFile)
  const projectName = path.basename(path.dirname(process.cwd()))

  return {
    context: `[${projectName}] ${relativePath}`,
    absolute: absolutePath,
    display: `[${projectName}] ${relativePath}\n  Full: ${absolutePath}`
  }
}

