import { m } from '@/paraglide/messages'

export function getErrorMessage(error: unknown): string {
  // Handle Error instances
  if (error instanceof Error) {
    return error.message
  }

  // Handle API error objects (common patterns)
  if (typeof error === 'object' && error !== null) {
    // Try common error message fields
    if ('message' in error && typeof error.message === 'string') {
      return error.message
    }
    if ('detail' in error && typeof error.detail === 'string') {
      return error.detail
    }
    if ('error_description' in error && typeof error.error_description === 'string') {
      return error.error_description
    }
    if ('error' in error && typeof error.error === 'string') {
      return error.error
    }
  }

  // Handle strings
  if (typeof error === 'string') {
    return error
  }

  // Fallback
  return m['error.generic']()
}

/**
 * API 错误处理函数
 * @param error API 错误对象
 * @param defaultMessage 默认错误消息
 * @returns 错误消息字符串
 */
export function handleApiError(error: unknown, defaultMessage?: string): string {
  const fallback = defaultMessage ?? m['error.generic']()

  console.error('[API Error]', error)

  let errorMessage = fallback

  // 处理不同类型的错误
  if (typeof error === 'string') {
    errorMessage = error
  } else if (error instanceof Error) {
    errorMessage = error.message
  } else if (error && typeof error === 'object') {
    if ('message' in error && typeof error.message === 'string') {
      errorMessage = error.message
    } else if ('detail' in error && typeof error.detail === 'string') {
      errorMessage = error.detail
    } else if ('status' in error && typeof error.status === 'number') {
      // HTTP 状态码错误
      switch (error.status) {
        case 400:
          errorMessage = m['error.bad_request']()
          break
        case 401:
          errorMessage = m['error.unauthorized']()
          break
        case 403:
          errorMessage = m['error.forbidden']()
          break
        case 404:
          errorMessage = m['error.not_found']()
          break
        case 409:
          errorMessage =
            'detail' in error && typeof error.detail === 'string'
              ? error.detail
              : m['error.conflict']()
          break
        case 500:
          errorMessage = m['error.server_error']()
          break
        default:
          errorMessage = fallback
      }
    }
  }

  return errorMessage
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
