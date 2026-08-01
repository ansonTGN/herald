/**
 * Subscription History Helpers
 *
 * Helper functions for subscription history timeline tests.
 * These helpers work with the UI and avoid direct database operations.
 */

// ============================================================================
// Timeline State Helpers
// ============================================================================

/**
 * 检查时间线是否为空
 */
export async function isTimelineEmpty(page: import('playwright').Page): Promise<boolean> {
  const emptyTimeline = page.getByTestId('subscription-timeline-empty')
  const emptySelector = page.getByTestId('subscription-selector-empty')
  const hasEmptyTimeline = await emptyTimeline.isVisible().catch(() => false)
  if (hasEmptyTimeline) return true
  const hasEmptySelector = await emptySelector.isVisible().catch(() => false)
  if (hasEmptySelector) return true
  return false
}

/**
 * 等待时间线加载完成
 */
export async function waitForTimelineToLoad(page: import('playwright').Page): Promise<void> {
  await page.waitForLoadState('networkidle')
  // 等待时间线容器或空状态出现
  await page.waitForSelector('[data-testid="subscription-timeline"], [data-testid="subscription-timeline-empty"]', {
    timeout: 5000
  }).catch(() => {
    // 如果选择器未找到，继续执行（可能是空状态）
  })
}

/**
 * 导航到订阅详情历史页面
 */
export async function navigateToSubscriptionDetailHistory(
  page: import('playwright').Page,
  realmId: string
): Promise<void> {
  await page.goto(`http://localhost:3000/user/subscription-history`)
  await page.waitForLoadState('networkidle')
  await waitForTimelineToLoad(page)
}

/**
 * 导航到用户个人页面
 */
export async function navigateToUserProfile(
  page: import('playwright').Page,
  realmId: string
): Promise<void> {
  await page.goto(`http://localhost:3000/user/profile`)
  await page.waitForLoadState('domcontentloaded')
  // Wait for profile content to render (networkidle may never resolve due to React Query retries on 404)
  await page.getByText('Profile Information').waitFor({ state: 'visible', timeout: 10000 }).catch(() => {
    // Fallback: wait for any heading to appear
  })
}
