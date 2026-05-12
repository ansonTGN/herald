/**
 * Backend API Validator for Demo Tests
 *
 * Provides comprehensive backend health validation with:
 * - Schema validation
 * - Error code analysis
 * - Request ID tracking for log correlation
 * - Detailed diagnostics with actionable suggestions
 *
 * Enhanced version of the simple health check in environment-setup.ts
 */

import { APIRequestContext, request } from '@playwright/test'

const API_BASE_URL = process.env.API_BASE_URL || 'http://localhost:8080'

/**
 * Backend health check response schema
 */
export interface BackendHealthResponse {
  status: 'healthy' | 'unhealthy' | 'degraded'
  database: boolean
  redis: boolean
  version?: string
  uptime?: number
  timestamp?: string
  error?: string
  error_code?: string
  details?: Record<string, any>
}

/**
 * Validation result with diagnostics
 */
export interface ValidationResult {
  success: boolean
  healthy: boolean
  response?: BackendHealthResponse
  errors: ValidationError[]
  warnings: ValidationWarning[]
  requestId?: string
  duration: number
}

/**
 * Validation error with actionable suggestions
 */
export interface ValidationError {
  code: string
  message: string
  field?: string
  suggestion: string
  severity: 'critical' | 'error' | 'warning'
}

/**
 * Validation warning (non-blocking)
 */
export interface ValidationWarning {
  code: string
  message: string
  suggestion: string
}

/**
 * Validation options
 */
export interface ValidationOptions {
  /** Maximum number of retry attempts */
  maxRetries?: number
  /** Initial retry delay in milliseconds */
  retryDelay?: number
  /** Request timeout in milliseconds */
  timeout?: number
  /** Whether to validate response schema strictly */
  strictSchema?: boolean
  /** Whether to skip request ID extraction */
  skipRequestId?: boolean
}

/**
 * Error code mappings to diagnostics
 */
const ERROR_CODE_MAP: Record<string, { message: string; suggestion: string }> = {
  DB_CONNECTION_FAILED: {
    message: 'Database connection failed',
    suggestion: 'Check PostgreSQL service: docker ps | grep cas-demo-postgres',
  },
  REDIS_CONNECTION_FAILED: {
    message: 'Redis connection failed',
    suggestion: 'Check Redis service: docker ps | grep cas-demo-redis',
  },
  DATABASE_QUERY_FAILED: {
    message: 'Database query execution failed',
    suggestion: 'Check database logs: tail -f log/backend-demo.log | grep -i error',
  },
  UNAUTHORIZED: {
    message: 'Authentication failed',
    suggestion: 'Check backend configuration: HERALD_CONFIG environment variable',
  },
}

/**
 * Validate backend health with comprehensive diagnostics
 *
 * @param options Validation options
 * @returns Validation result with detailed diagnostics
 *
 * @example
 * ```typescript
 * const result = await validateBackendHealth({
 *   maxRetries: 3,
 *   retryDelay: 1000,
 *   timeout: 5000,
 * })
 *
 * if (!result.success) {
 *   console.error(formatValidationErrors(result))
 * }
 * ```
 */
export async function validateBackendHealth(
  options: ValidationOptions = {}
): Promise<ValidationResult> {
  const {
    maxRetries = 3,
    retryDelay = 1000,
    timeout = 5000,
    strictSchema = true,
    skipRequestId = false,
  } = options

  const startTime = Date.now()
  const errors: ValidationError[] = []
  const warnings: ValidationWarning[] = []
  let response: BackendHealthResponse | undefined
  let requestId: string | undefined
  let lastError: Error | null = null

  // Retry loop with exponential backoff
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      // Create API request context
      const apiContext = await request.newContext({
        baseURL: API_BASE_URL,
        timeout,
      })

      // Make health check request
      const apiResponse = await apiContext.get('/health', {
        headers: {
          'Accept': 'application/json',
          'User-Agent': 'Herald-Demo-Test/1.0',
        },
      })

      // Extract request ID from response headers
      if (!skipRequestId) {
        requestId = apiResponse.headers()['x-request-id'] ||
                     apiResponse.headers()['x-cas-request-id'] ||
                     generateRequestId()
      }

      // Parse response body
      const bodyText = await apiResponse.text()
      let body: any

      try {
        body = JSON.parse(bodyText)
      } catch (parseError) {
        throw new Error(`Invalid JSON response: ${bodyText.substring(0, 200)}`)
      }

      // Validate HTTP status
      if (apiResponse.status() !== 200) {
        throw new Error(`HTTP ${apiResponse.status()}: ${bodyText}`)
      }

      // Validate response schema
      const schemaErrors = validateHealthSchema(body, strictSchema)
      if (schemaErrors.length > 0) {
        errors.push(...schemaErrors)
      }

      response = body as BackendHealthResponse

      // Check health status
      if (response.status !== 'healthy') {
        // Analyze error codes
        if (response.error_code) {
          const errorInfo = ERROR_CODE_MAP[response.error_code]
          if (errorInfo) {
            errors.push({
              code: response.error_code,
              message: errorInfo.message,
              suggestion: errorInfo.suggestion,
              severity: 'critical',
            })
          } else {
            errors.push({
              code: 'UNKNOWN_ERROR',
              message: response.error || 'Unknown error',
              suggestion: 'Check backend logs for details',
              severity: 'critical',
            })
          }
        }

        // Check individual components
        if (!response.database) {
          errors.push({
            code: 'DB_CONNECTION_FAILED',
            message: 'Database connection check failed',
            suggestion: 'Check PostgreSQL: docker ps | grep postgres',
            severity: 'critical',
          })
        }

        if (!response.redis) {
          errors.push({
            code: 'REDIS_CONNECTION_FAILED',
            message: 'Redis connection check failed',
            suggestion: 'Check Redis: docker ps | grep redis',
            severity: 'critical',
          })
        }
      }

      // Check for optional fields
      if (!response.version) {
        warnings.push({
          code: 'MISSING_VERSION',
          message: 'Backend version not provided in health response',
          suggestion: 'Consider adding version field to health endpoint',
        })
      }

      // Success - break retry loop
      await apiContext.dispose()
      break

    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error))

      if (attempt < maxRetries) {
        const backoffDelay = retryDelay * Math.pow(2, attempt - 1)
        console.warn(`  Health check failed, retrying in ${backoffDelay}ms... (${attempt}/${maxRetries})`)
        await new Promise(resolve => setTimeout(resolve, backoffDelay))
      } else {
        // All retries exhausted
        errors.push({
          code: 'HEALTH_CHECK_FAILED',
          message: `Health check failed after ${maxRetries} attempts: ${lastError.message}`,
          suggestion: 'Check backend service status: curl http://localhost:8080/health',
          severity: 'critical',
        })
      }
    }
  }

  const duration = Date.now() - startTime
  const success = errors.length === 0
  const healthy = response?.status === 'healthy'

  return {
    success,
    healthy,
    response,
    errors,
    warnings,
    requestId,
    duration,
  }
}

/**
 * Validate health response schema
 *
 * @param body Response body
 * @param strict Whether to use strict validation
 * @returns List of validation errors
 */
function validateHealthSchema(
  body: any,
  strict: boolean
): ValidationError[] {
  const errors: ValidationError[] = []

  // Check required fields
  if (typeof body.status !== 'string') {
    errors.push({
      code: 'INVALID_SCHEMA',
      message: 'Missing or invalid "status" field',
      field: 'status',
      suggestion: 'Backend health endpoint must return status field',
      severity: 'error',
    })
  } else if (!['healthy', 'unhealthy', 'degraded'].includes(body.status)) {
    errors.push({
      code: 'INVALID_SCHEMA',
      message: `Invalid status value: "${body.status}"`,
      field: 'status',
      suggestion: 'Status must be one of: healthy, unhealthy, degraded',
      severity: 'error',
    })
  }

  if (typeof body.database !== 'boolean') {
    errors.push({
      code: 'INVALID_SCHEMA',
      message: 'Missing or invalid "database" field',
      field: 'database',
      suggestion: 'Backend health endpoint must return database field (boolean)',
      severity: 'error',
    })
  }

  if (typeof body.redis !== 'boolean') {
    errors.push({
      code: 'INVALID_SCHEMA',
      message: 'Missing or invalid "redis" field',
      field: 'redis',
      suggestion: 'Backend health endpoint must return redis field (boolean)',
      severity: 'error',
    })
  }

  // Optional fields (warnings only)
  if (strict && !body.version) {
    errors.push({
      code: 'MISSING_OPTIONAL_FIELD',
      message: 'Missing "version" field (recommended)',
      field: 'version',
      suggestion: 'Add version field to health endpoint for better diagnostics',
      severity: 'warning' as any,
    })
  }

  return errors
}

/**
 * Format validation errors for human-readable output
 *
 * @param result Validation result
 * @returns Formatted error string
 *
 * @example
 * ```typescript
 * const result = await validateBackendHealth()
 * if (!result.success) {
 *   console.error(formatValidationErrors(result))
 * }
 * ```
 */
export function formatValidationErrors(result: ValidationResult): string {
  const lines: string[] = []

  lines.push('='.repeat(60))
  lines.push('Backend Health Validation Failed')
  lines.push('='.repeat(60))
  lines.push('')

  if (result.requestId) {
    lines.push(`Request ID: ${result.requestId}`)
    lines.push(`Duration: ${result.duration}ms`)
    lines.push('')
  }

  // Critical errors
  const criticalErrors = result.errors.filter(e => e.severity === 'critical')
  if (criticalErrors.length > 0) {
    lines.push('CRITICAL ERRORS:')
    lines.push('-'.repeat(60))
    for (const error of criticalErrors) {
      lines.push(`  [${error.code}] ${error.message}`)
      lines.push(`  → ${error.suggestion}`)
      lines.push('')
    }
  }

  // Regular errors
  const regularErrors = result.errors.filter(e => e.severity === 'error')
  if (regularErrors.length > 0) {
    lines.push('ERRORS:')
    lines.push('-'.repeat(60))
    for (const error of regularErrors) {
      if (error.field) {
        lines.push(`  [${error.code}] ${error.field}: ${error.message}`)
      } else {
        lines.push(`  [${error.code}] ${error.message}`)
      }
      lines.push(`  → ${error.suggestion}`)
      lines.push('')
    }
  }

  // Warnings
  if (result.warnings.length > 0) {
    lines.push('WARNINGS:')
    lines.push('-'.repeat(60))
    for (const warning of result.warnings) {
      lines.push(`  [${warning.code}] ${warning.message}`)
      lines.push(`  → ${warning.suggestion}`)
      lines.push('')
    }
  }

  // Response info
  if (result.response) {
    lines.push('RESPONSE:')
    lines.push('-'.repeat(60))
    lines.push(`  Status: ${result.response.status}`)
    lines.push(`  Database: ${result.response.database ? '✓' : '✗'}`)
    lines.push(`  Redis: ${result.response.redis ? '✓' : '✗'}`)
    if (result.response.version) {
      lines.push(`  Version: ${result.response.version}`)
    }
    if (result.response.uptime) {
      lines.push(`  Uptime: ${result.response.uptime}s`)
    }
    lines.push('')
  }

  // Quick commands
  lines.push('QUICK DIAGNOSTICS:')
  lines.push('-'.repeat(60))
  lines.push('  # Check backend health manually')
  lines.push('  curl http://localhost:8080/health')
  lines.push('')
  lines.push('  # Check backend logs')
  lines.push('  tail -f log/backend-demo.log')
  lines.push('')
  lines.push('  # Restart demo environment')
  lines.push('  pwsh -File scripts/demo-stop.ps1')
  lines.push('  pwsh -File scripts/demo-start.ps1')
  lines.push('')

  lines.push('='.repeat(60))

  return lines.join('\n')
}

/**
 * Generate a request ID for tracking
 */
function generateRequestId(): string {
  const timestamp = Date.now().toString(36)
  const random = Math.random().toString(36).substring(2, 9)
  return `req-${timestamp}-${random}`
}

/**
 * Quick health check (simplified version for compatibility)
 *
 * @returns Promise that resolves if backend is healthy
 *
 * @example
 * ```typescript
 * try {
 *   await quickHealthCheck()
 *   console.log('Backend is healthy')
 * } catch (error) {
 *   console.error('Backend health check failed:', error)
 * }
 * ```
 */
export async function quickHealthCheck(): Promise<void> {
  const result = await validateBackendHealth({
    maxRetries: 1,
    retryDelay: 500,
    timeout: 3000,
    strictSchema: false,
  })

  if (!result.healthy) {
    throw new Error(formatValidationErrors(result))
  }
}
