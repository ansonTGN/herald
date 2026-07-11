import { m } from '@/paraglide/messages'

export interface ResolvedApiError {
  status?: number
  code?: string
  message?: string
  details?: unknown
  requestId?: string
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null
    ? (value as Record<string, unknown>)
    : undefined
}

export function resolveApiError(error: unknown): ResolvedApiError {
  if (error instanceof ApiResponseError) {
    return {
      status: error.status,
      code: error.code,
      message: error.message,
      details: error.details,
      requestId: error.requestId,
    }
  }

  if (error instanceof Error) return { message: error.message }
  if (typeof error === 'string') return { message: error }

  const outer = asRecord(error)
  if (!outer) return {}
  const nested = asRecord(outer.error)
  const value = nested ?? outer
  const status =
    typeof value.status === 'number'
      ? value.status
      : typeof outer.status === 'number'
        ? outer.status
        : undefined

  return {
    status,
    code: typeof value.code === 'string' ? value.code : undefined,
    message:
      typeof value.message === 'string'
        ? value.message
        : typeof value.detail === 'string'
          ? value.detail
          : typeof value.error_description === 'string'
            ? value.error_description
            : typeof value.error === 'string'
              ? value.error
              : undefined,
    details: value.details,
    requestId:
      typeof value.requestId === 'string'
        ? value.requestId
        : typeof value.request_id === 'string'
          ? value.request_id
          : undefined,
  }
}

export class ApiResponseError extends Error {
  readonly status?: number
  readonly code?: string
  readonly details?: unknown
  readonly requestId?: string

  constructor(error: unknown) {
    const resolved = resolveApiError(error)
    super(resolved.message ?? m['error.generic']())
    this.name = 'ApiResponseError'
    this.status = resolved.status
    this.code = resolved.code
    this.details = resolved.details
    this.requestId = resolved.requestId
  }
}

export function getErrorMessage(error: unknown): string {
  const resolved = resolveApiError(error)
  const message =
    resolved.status !== undefined && resolved.status >= 500
      ? m['error.server_error']()
      : (resolved.message ?? m['error.generic']())

  return resolved.requestId ? `${message} (${resolved.requestId})` : message
}

/**
 * API 错误处理函数
 * @param error API 错误对象
 * @param defaultMessage 默认错误消息
 * @returns 错误消息字符串
 */
export function handleApiError(error: unknown, defaultMessage?: string): string {
  console.error('[API Error]', error)
  const resolved = resolveApiError(error)
  if (!resolved.message && defaultMessage) return defaultMessage
  return getErrorMessage(error)
}

/**
 * Extract error message from TanStack Form field error
 * Handles Zod validation errors and other error types
 */
export function getFieldErrorMessage(error: unknown): string {
  if (typeof error === 'string') {
    return error
  }

  if (error && typeof error === 'object') {
    // Handle Zod error objects
    if ('message' in error && typeof error.message === 'string') {
      return error.message
    }
  }

  return String(error ?? '')
}

/**
 * 处理表单验证错误
 * @param errors Zod 验证错误对象
 */
export function handleFormErrors(errors: Record<string, string[]>) {
  const firstError = Object.values(errors)[0]?.[0]
  if (firstError) {
    console.error('[Form Validation Error]', firstError)
  }
}
