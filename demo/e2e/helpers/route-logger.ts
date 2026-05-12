/**
 * 路由变化监听器 - 用于记录演示测试中的页面导航
 *
 * 用途：
 * - 捕获页面路由变化（导航历史）
 * - 诊断自动重定向问题（如 Session 持久化导致的重定向）
 * - 追踪用户操作路径
 *
 * 使用方式：
 * ```typescript
 * import { UnifiedLogger } from '../../helpers/unified-logger'
 *
 * test.afterEach(async ({ logger }) => {
 *   logger.route.printRouteChanges()
 * })
 * ```
 */

import { Page } from '@playwright/test'

export interface RouteChangeLog {
  timestamp: string
  url: string
  title?: string
  trigger?: 'goto' | 'redirect' | 'back_forward' | 'unknown'
}

export class RouteLogger {
  private routeLogs: RouteChangeLog[] = []
  private page!: Page
  private quietMode: boolean
  private lastUrl: string = ''

  constructor(page: Page, quietMode: boolean = false) {
    this.page = page
    this.quietMode = quietMode
    this.attachListeners()
  }

  private attachListeners() {
    // 监听页面导航（路由变化）
    this.page.on('framenavigated', async (frame) => {
      // 只记录主 frame 的导航（忽略 iframe）
      if (frame === this.page.mainFrame()) {
        const url = frame.url()
        const title = await frame.title().catch(() => '')

        // 避免记录重复的 URL（hash 变化等）
        if (url !== this.lastUrl) {
          const log: RouteChangeLog = {
            timestamp: new Date().toISOString(),
            url: url,
            title: title || undefined,
            trigger: this.detectTriggerType(url)
          }

          this.routeLogs.push(log)
          this.lastUrl = url

          // 在 quiet 模式下也打印路由变化（重要调试信息）
          if (this.quietMode) {
            const triggerIcon = this.getTriggerIcon(log.trigger)
            console.log(`[Route] ${triggerIcon} ${url}`)
          }
        }
      }
    })
  }

  /**
   * 检测路由触发类型
   * 注意：Playwright 无法直接区分导航类型，这里使用启发式判断
   */
  private detectTriggerType(url: string): 'goto' | 'redirect' | 'back_forward' | 'unknown' {
    // 简单的启发式判断
    if (url.includes('/redirect') || url.includes('?redirect')) {
      return 'redirect'
    }
    return 'unknown'
  }

  /**
   * 获取触发类型图标
   */
  private getTriggerIcon(trigger?: string): string {
    switch (trigger) {
      case 'goto': return '→'
      case 'redirect': return '↝'
      case 'back_forward': return '↔'
      default: return '→'
    }
  }

  /**
   * 打印路由变化日志
   */
  printRouteChanges(title?: string) {
    if (this.routeLogs.length === 0) {
      console.log('[Route] No route changes captured')
      return
    }

    console.log('\n' + '='.repeat(80))
    console.log(title || 'Route Changes')
    console.log('='.repeat(80))

    for (let i = 0; i < this.routeLogs.length; i++) {
      const log = this.routeLogs[i]
      const triggerIcon = this.getTriggerIcon(log.trigger)
      const stepNum = i + 1

      console.log(`\n[${stepNum}] ${log.timestamp}`)

      if (log.title) {
        console.log(`  ${triggerIcon} ${log.url}`)
        console.log(`     Title: "${log.title}"`)
      } else {
        console.log(`  ${triggerIcon} ${log.url}`)
      }

      if (log.trigger) {
        console.log(`     Trigger: ${log.trigger}`)
      }

      // 检测可能的自动重定向
      if (i > 0 && this.isPossibleRedirect(this.routeLogs[i - 1].url, log.url)) {
        console.log(`     ⚠️  Possible auto-redirect detected`)
      }
    }

    console.log('\n' + '='.repeat(80))
  }

  /**
   * 检测是否为可能的自动重定向
   */
  private isPossibleRedirect(fromUrl: string, toUrl: string): boolean {
    // 从登录/注册页面重定向到 Dashboard
    const isFromAuth = fromUrl.includes('/login') || fromUrl.includes('/register')
    const isToDashboard = toUrl.includes('/dashboard') || toUrl.includes('/manage')

    if (isFromAuth && isToDashboard) {
      return true
    }

    return false
  }

  /**
   * Mini summary for quiet mode
   */
  printMiniSummary(title?: string) {
    const redirectCount = this.detectRedirectCount()

    console.log(`[Route] ${title || 'Summary'}: ${this.routeLogs.length} changes${redirectCount > 0 ? ` (${redirectCount} possible redirects)` : ''}`)

    // 显示最近 3 次路由变化
    if (this.routeLogs.length > 0) {
      const recent = this.routeLogs.slice(-3)
      recent.forEach((log, idx) => {
        const triggerIcon = this.getTriggerIcon(log.trigger)
        const prefix = this.routeLogs.length > 3
          ? `... +${this.routeLogs.length - 3} more, then `
          : `${this.routeLogs.length - recent.length + idx + 1}. `
        console.log(`  ${prefix}${triggerIcon} ${this.shortenUrl(log.url)}`)
      })
    }
  }

  /**
   * 缩短 URL 显示
   */
  private shortenUrl(url: string): string {
    try {
      const urlObj = new URL(url)
      const path = urlObj.pathname
      // 只显示最后两段路径
      const segments = path.split('/').filter(s => s)
      if (segments.length > 2) {
        return '.../' + segments.slice(-2).join('/')
      }
      return path || url
    } catch {
      return url.length > 50 ? url.substring(0, 50) + '...' : url
    }
  }

  /**
   * 检测可能的自动重定向次数
   */
  private detectRedirectCount(): number {
    let count = 0
    for (let i = 1; i < this.routeLogs.length; i++) {
      if (this.isPossibleRedirect(this.routeLogs[i - 1].url, this.routeLogs[i].url)) {
        count++
      }
    }
    return count
  }

  /**
   * 获取路由日志
   */
  getRouteLogs(): RouteChangeLog[] {
    return this.routeLogs
  }

  /**
   * 清空路由日志
   */
  clearRouteLogs() {
    this.routeLogs = []
    this.lastUrl = ''
  }

  /**
   * 导出为 JSON
   */
  exportToJson(): string {
    return JSON.stringify(this.routeLogs, null, 2)
  }

  /**
   * 分析路由模式
   */
  analyzeRoutePattern(): {
    totalChanges: number
    possibleRedirects: number
    authPages: number
    dashboardPages: number
    lastUrl: string
  } {
    const authPages = this.routeLogs.filter(log =>
      log.url.includes('/login') || log.url.includes('/register')
    ).length

    const dashboardPages = this.routeLogs.filter(log =>
      log.url.includes('/dashboard')
    ).length

    return {
      totalChanges: this.routeLogs.length,
      possibleRedirects: this.detectRedirectCount(),
      authPages,
      dashboardPages,
      lastUrl: this.routeLogs.length > 0 ? this.routeLogs[this.routeLogs.length - 1].url : ''
    }
  }
}
