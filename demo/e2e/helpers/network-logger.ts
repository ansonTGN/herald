/**
 * 网络请求监听器 - 用于记录演示测试中的 API 请求
 *
 * 用途：
 * - 捕获所有 API 请求和响应
 * - 在测试失败时提供详细的调试信息
 * - 帮助 AI 定位前后端交互问题
 *
 * ✅ Cleaned: Removed unused testWithNetworkLogger fixture
 */

import { Page } from '@playwright/test'

export interface ApiRequestLog {
  timestamp: string
  method: string
  url: string
  status?: number
  contentType?: string
  requestHeaders?: Record<string, string>
  requestCookie?: string
  requestBodyRaw?: string
  requestBody?: string
  bodyRedacted?: boolean
  responseBody?: string
  duration?: number
  error?: string
  requestId?: string  // Request ID for correlating frontend and backend logs
  responseSetCookie?: string  // Set-Cookie header from response (for auth cookie tracking)
  pageCookies?: string  // Current cookies from page context (for replay)
}

export class NetworkLogger {
  private logs: ApiRequestLog[] = []
  private page!: Page
  private requestStartTimes: Map<string, number> = new Map()
  private quietMode: boolean

  constructor(page: Page, quietMode: boolean = false) {
    this.page = page
    this.quietMode = quietMode
    this.attachListeners()
  }

  private attachListeners() {
    // 监听所有请求
    this.page.on('request', async (request) => {
      const url = request.url()
      // 只记录 API 请求
      if (this.isApiRequest(url)) {
        const headers = request.headers()
        const requestCookie = headers['cookie'] || headers['Cookie']
        const requestHeaders = this.pickReplayHeaders(headers)

        // 获取当前 page 的所有 cookies（用于诊断和 replay）
        let pageCookies = ''
        try {
          const cookies = await this.page.context().cookies()
          // 格式化为 Cookie 头字符串
          pageCookies = cookies
            .map(c => `${c.name}=${c.value}`)
            .join('; ')
        } catch {
          // 忽略错误，有些情况下无法获取 cookies
        }

        const log: ApiRequestLog = {
          timestamp: new Date().toISOString(),
          method: request.method(),
          url: url,
          contentType: headers['content-type'],
          requestId: headers['x-request-id'],  // Capture request ID from client request
          requestHeaders,
          requestCookie,
          pageCookies,  // 页面级别的 cookies（包含 HttpOnly cookies）
        }

        // 记录请求开始时间
        this.requestStartTimes.set(url, Date.now())

        // 记录请求体（排除登录密码等敏感信息）
        try {
          const postData = request.postData()
          if (postData) {
            log.requestBodyRaw = postData
            log.requestBody = this.sanitizeData(postData)
            log.bodyRedacted = log.requestBody !== postData
          }
        } catch {
          // 忽略解析错误
        }

        this.logs.push(log)
      }
    })

    // 监听所有响应
    this.page.on('response', async (response) => {
      const url = response.url()
      if (this.isApiRequest(url)) {
        const log = this.logs.find(
          (l) => l.url === url && !l.status
        )

        if (log) {
          const responseHeaders = response.headers()
          log.status = response.status()
          log.contentType = responseHeaders['content-type']

          // 记录 Set-Cookie 头（用于跟踪 auth cookie 设置）
          if (responseHeaders['set-cookie']) {
            log.responseSetCookie = this.sanitizeCookie(responseHeaders['set-cookie'])
          }

          // Use the response's X-Request-ID as the authoritative source
          // (server either returns the client's request_id or generates a new one)
          if (responseHeaders['x-request-id']) {
            log.requestId = responseHeaders['x-request-id']
          }

          // 计算请求耗时
          const startTime = this.requestStartTimes.get(url)
          if (startTime) {
            log.duration = Date.now() - startTime
            this.requestStartTimes.delete(url)
          }

          try {
            // 记录响应体（限制大小）
            const body = await response.text()
            if (body && body.length < 5000) {
              log.responseBody = body
            } else if (body) {
              log.responseBody = body.substring(0, 5000) + '... (truncated)'
            }
          } catch {
            // 忽略解析错误
          }
        }
      }
    })

    // 监听请求失败
    this.page.on('requestfailed', (request) => {
      const url = request.url()
      if (this.isApiRequest(url)) {
        const log = this.logs.find(
          (l) => l.url === url && !l.error
        )

        if (log) {
          log.error = request.failure()?.errorText || 'Unknown error'

          // 计算请求耗时
          const startTime = this.requestStartTimes.get(url)
          if (startTime) {
            log.duration = Date.now() - startTime
            this.requestStartTimes.delete(url)
          }
        }
      }
    })
  }

  /**
   * 判断是否为 API 请求
   */
  private isApiRequest(url: string): boolean {
    return url.includes('/api/') || url.includes('/_api/')
  }

  /**
   * 脱敏处理：隐藏密码等敏感信息
   */
  private sanitizeData(data: string): string {
    try {
      const parsed = JSON.parse(data)
      const sanitized = JSON.parse(JSON.stringify(parsed), (key, value) => {
        if (typeof value === 'string' && (
          key.toLowerCase().includes('password') ||
          key.toLowerCase().includes('secret') ||
          key.toLowerCase().includes('token') ||
          key.toLowerCase().includes('currentpassword') ||
          key.toLowerCase().includes('newpassword')
        )) {
          return '***REDACTED***'
        }
        return value
      })
      return JSON.stringify(sanitized)
    } catch {
      // 如果不是 JSON，简单替换密码字段
      return data.replace(/"password":"[^"]*"/g, '"password":"***REDACTED***"')
    }
  }

  /**
   * Cookie 脱敏处理：隐藏敏感 cookie 值但保留结构
   */
  private sanitizeCookie(setCookie: string): string {
    // 分割多个 set-cookie 头（多个 cookie 用换行分隔）
    const cookies = setCookie.split('\n')
    return cookies.map(cookie => {
      // 匹配 cookie 名称和值的模式: name=value
      const match = cookie.match(/^([^=]+)=([^;]+)/)
      if (match) {
        const name = match[1]
        // 对认证相关的 cookie 脱敏，其他 cookie 保留
        const authCookieNames = ['x-auth', 'x-auth-refresh', 'token', 'session', 'sid', 'jwt']
        if (authCookieNames.includes(name.toLowerCase())) {
          return cookie.replace(match[2], '***REDACTED***')
        }
      }
      return cookie
    }).join('\n')
  }

  /**
   * Pick request headers useful for API replay.
   */
  private pickReplayHeaders(headers: Record<string, string>): Record<string, string> {
    const keep = [
      'content-type',
      'accept',
      'authorization',
      'x-request-id',
      'cookie',
      'user-agent',
    ]
    const selected: Record<string, string> = {}
    for (const [key, value] of Object.entries(headers)) {
      const lowerKey = key.toLowerCase()
      if (keep.includes(lowerKey)) {
        selected[key] = value
      }
    }
    return selected
  }

  /**
   * 打印日志到控制台
   */
  printLogs(title?: string) {
    if (this.logs.length === 0) {
      console.log('[Network] No API requests captured')
      return
    }

    console.log('\n' + '='.repeat(80))
    console.log(title || 'API Request Logs')
    console.log('='.repeat(80))

    for (const log of this.logs) {
      console.log(`\n[${log.timestamp}] ${log.method} ${log.url}`)
      if (log.requestId) {
        console.log(`  Request ID: ${log.requestId}`)
      }
      if (log.requestCookie) {
        console.log(`  Cookie: ${log.requestCookie}`)
      }
      if (log.pageCookies) {
        console.log(`  Page Cookies: ${log.pageCookies}`)
      }

      if (log.requestBody) {
        console.log(`  → Request: ${log.requestBody}`)
      }

      if (log.status) {
        const statusIcon = log.status >= 200 && log.status < 300 ? '✓' : '✗'
        console.log(`  ← Response: ${statusIcon} ${log.status}${log.duration ? ` (${log.duration}ms)` : ''}`)

        if (log.responseSetCookie) {
          console.log(`  Set-Cookie: ${log.responseSetCookie}`)
        }

        if (log.responseBody) {
          // 格式化 JSON
          try {
            const parsed = JSON.parse(log.responseBody)
            const formatted = JSON.stringify(parsed, null, 2)
            console.log(`    Body:\n${formatted.split('\n').join('\n    ')}`)
          } catch {
            console.log(`    Body: ${log.responseBody}`)
          }
        }
      }

      if (log.error) {
        console.log(`  ✗ Error: ${log.error}${log.duration ? ` (${log.duration}ms)` : ''}`)
      }
    }

    console.log('\n' + '='.repeat(80))
  }

  /**
   * 打印失败的请求
   */
  printFailedLogs(title?: string) {
    const failedLogs = this.logs.filter(log =>
      log.error || (log.status && log.status >= 400)
    )

    if (failedLogs.length === 0) {
      console.log('[Network] All API requests successful')
      return
    }

    console.log('\n' + '='.repeat(80))
    console.log(title || 'Failed API Requests')
    console.log('='.repeat(80))

    for (const log of failedLogs) {
      console.log(`\n[${log.timestamp}] ${log.method} ${log.url}`)
      if (log.requestCookie) {
        console.log(`  Cookie: ${log.requestCookie}`)
      }
      if (log.pageCookies) {
        console.log(`  Page Cookies: ${log.pageCookies}`)
      }

      if (log.requestBody) {
        console.log(`  → Request: ${log.requestBody}`)
      }

      if (log.status) {
        console.log(`  ← Response: ✗ ${log.status}${log.duration ? ` (${log.duration}ms)` : ''}`)

        if (log.responseSetCookie) {
          console.log(`  Set-Cookie: ${log.responseSetCookie}`)
        }

        if (log.responseBody) {
          try {
            const parsed = JSON.parse(log.responseBody)
            const formatted = JSON.stringify(parsed, null, 2)
            console.log(`    Body:\n${formatted.split('\n').join('\n    ')}`)
          } catch {
            console.log(`    Body: ${log.responseBody}`)
          }
        }
      }

      if (log.error) {
        console.log(`  ✗ Error: ${log.error}${log.duration ? ` (${log.duration}ms)` : ''}`)
      }
    }

    console.log('\n' + '='.repeat(80))
  }

  /**
   * 获取日志
   */
  getLogs(): ApiRequestLog[] {
    return this.logs
  }

  /**
   * 清空日志
   */
  clearLogs() {
    this.logs = []
    this.requestStartTimes.clear()
  }

  /**
   * 导出为 JSON
   */
  exportToJson(): string {
    return JSON.stringify(this.logs, null, 2)
  }

  /**
   * 获取失败的日志
   */
  getFailedLogs(): ApiRequestLog[] {
    return this.logs.filter(log =>
      log.error || (log.status && log.status >= 400)
    )
  }

  /**
   * 根据 request_id 查找日志
   */
  findLogsByRequestId(requestId: string): ApiRequestLog[] {
    return this.logs.filter(log => log.requestId === requestId)
  }

  /**
   * 获取所有包含 request_id 的日志
   */
  getLogsWithRequestId(): ApiRequestLog[] {
    return this.logs.filter(log => log.requestId)
  }

  /**
   * Mini summary for quiet mode - Single line output to reduce token usage
   */
  printMiniSummary(title?: string) {
    const failed = this.getFailedLogs()
    const aggregated = this.getAggregatedLogs()
    const uniqueRequests = Object.keys(aggregated).length

    console.log(`[Network] ${title || 'Summary'}: ${this.logs.length} requests (${uniqueRequests} unique), ${failed.length} failed`)

    // Only show first 5 failed requests
    if (failed.length > 0 && failed.length <= 5) {
      failed.forEach(f => {
        const status = f.status || (f.error ? 'ERR' : '?')
        console.log(`  ${f.method} ${f.url} → ${status}`)
      })
    } else if (failed.length > 5) {
      console.log(`  (First 5 of ${failed.length} failed requests)`)
      failed.slice(0, 5).forEach(f => {
        const status = f.status || (f.error ? 'ERR' : '?')
        console.log(`  ${f.method} ${f.url} → ${status}`)
      })
    }
  }

  /**
   * Aggregate similar requests to reduce log verbosity
   * Groups requests by method + URL pattern
   */
  getAggregatedLogs(): Record<string, { count: number; successCount: number; failureCount: number; avgDuration?: number }> {
    const aggregated: Record<string, { count: number; successCount: number; failureCount: number; totalDuration: number }> = {}

    for (const log of this.logs) {
      // Create a key by method and URL path (ignore query params for aggregation)
      const url = new URL(log.url, 'http://localhost')
      const path = url.pathname
      const key = `${log.method} ${path}`

      if (!aggregated[key]) {
        aggregated[key] = {
          count: 0,
          successCount: 0,
          failureCount: 0,
          totalDuration: 0
        }
      }

      aggregated[key].count++
      if (log.status && log.status >= 200 && log.status < 300) {
        aggregated[key].successCount++
      } else if (log.status && log.status >= 400 || log.error) {
        aggregated[key].failureCount++
      }
      if (log.duration) {
        aggregated[key].totalDuration += log.duration
      }
    }

    // Convert to final format with average duration
    const result: Record<string, { count: number; successCount: number; failureCount: number; avgDuration?: number }> = {}
    for (const [key, value] of Object.entries(aggregated)) {
      result[key] = {
        count: value.count,
        successCount: value.successCount,
        failureCount: value.failureCount,
        avgDuration: value.totalDuration > 0 ? Math.round(value.totalDuration / value.count) : undefined
      }
    }

    return result
  }

  /**
   * Print aggregated logs summary (for analysis)
   */
  printAggregatedLogs(title?: string) {
    const aggregated = this.getAggregatedLogs()
    const entries = Object.entries(aggregated).sort((a, b) => b[1].count - a[1].count)

    console.log('\n' + '='.repeat(80))
    console.log(title || 'Aggregated API Requests')
    console.log('='.repeat(80))

    for (const [key, stats] of entries) {
      const statusIcon = stats.failureCount === 0 ? '✓' : '✗'
      const durationStr = stats.avgDuration ? ` (${stats.avgDuration}ms avg)` : ''
      console.log(`  ${statusIcon} ${key}: ${stats.count}x [${stats.successCount} success, ${stats.failureCount} failed]${durationStr}`)
    }

    console.log('='.repeat(80))
  }
}
