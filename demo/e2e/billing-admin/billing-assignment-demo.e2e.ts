/**
 * Billing Assignment Demo Tests
 *
 * User Story:
 * - US-BI-004: Assign Plan to Client App
 *
 * Design Doc: .ai/design/subscription-billing.md
 *
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { createBillingPlan, openAssignPlanDialog } from './helpers/billing-page.helpers'
import { DEMO_ADMIN } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'

test.describe('[Billing Admin] Assign Plan to Client App Demo Tests', () => {
  // Verify test environment before each test
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
  })

  // Single test.afterEach for cleanup
  test.afterEach(async ({ page, testStartTime }) => {
    // ⚠️ MANDATORY: 清理测试数据
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
  })

  // ============================================================================
  // User Story US-BI-004: Assign Plan to Client App
  // ============================================================================

  test.describe('US-BI-004: Assign Plan to Client App', () => {
    test('should assign plan to single client app', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planName = `assign-plan-${testStartTime}`

      await test.step('Given: 已存在一个订阅套餐', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        // 创建套餐
        await createBillingPlan(page, {
          planName,
          title: 'Assign Test Plan',
          price: '1000',
          type: 'monthly',
        })
        // Note: We don't wait for the plan to appear in the list because new plans
        // are ordered by sort_order=0 and may not appear on the first page
      })

      await test.step('When: 点击 "Assign to App" 按钮', async () => {
        await openAssignPlanDialog(page, planName)
        demoLogger.testCode.log('[Test] ✓ Assign to App button clicked')
      })

      await test.step('Then: 验证 Assignment Dialog 显示', async () => {
        await expect(page.getByTestId('plan-assignment-dialog-title')).toBeVisible()
        demoLogger.testCode.log('[Test] ✓ Assignment dialog displayed')
      })

      await test.step('When: 选择一个 Client App', async () => {
        // 等待复选框可见
        await expect(page.getByTestId(/^plan-assignment-checkbox-/)).toBeVisible({ timeout: 5000 })
        // 选择第一个 Client App
        await page.getByTestId(/^plan-assignment-checkbox-/).first().check()
        demoLogger.testCode.log('[Test] ✓ Client app selected')
      })

      await test.step('And: 点击 "Assign Plan" 按钮', async () => {
        await page.getByTestId('plan-assignment-submit-button').click()
      })

      await test.step('Then: 验证分配成功', async () => {
        // 验证 Assignment Dialog 关闭
        await expect(page.getByTestId('plan-assignment-dialog')).not.toBeVisible()
        demoLogger.testCode.log('[Test] ✓ Plan assigned successfully')
      })
    })

    test('should assign plan to multiple client apps', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planName = `multi-assign-test-${testStartTime}`

      await test.step('Given: 已存在一个订阅套餐', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        // 创建套餐
        await createBillingPlan(page, {
          planName,
          title: 'Multi Assign Test Plan',
          price: '1000',
          type: 'monthly',
        })
        // Note: Plan may not be visible on first page due to sort_order
      })

      await test.step('When: 点击 "Assign to App" 按钮', async () => {
        await openAssignPlanDialog(page, planName)
      })

      await test.step('And: 选择多个 Client App', async () => {
        // 等待复选框可见
        await expect(page.getByTestId(/^plan-assignment-checkbox-/)).toBeVisible({ timeout: 5000 })
        const checkboxes = page.getByTestId(/^plan-assignment-checkbox-/)
        const checkboxCount = await checkboxes.count()
        await expect(checkboxCount).toBeGreaterThan(0)

        await checkboxes.nth(0).check()
        if (checkboxCount > 1) {
          await checkboxes.nth(1).check()
        }
        demoLogger.testCode.log(`[Test] ✓ Selected ${Math.min(checkboxCount, 2)} client app(s)`)
      })

      await test.step('And: 点击 "Assign Plan" 按钮', async () => {
        await page.getByTestId('plan-assignment-submit-button').click()
      })

      await test.step('Then: 验证分配成功', async () => {
        await expect(page.getByTestId('plan-assignment-dialog')).not.toBeVisible()
        demoLogger.testCode.log('[Test] ✓ Plan assigned to multiple apps successfully')
      })
    })

    test('should view plan assignment status', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planName = `assign-status-test-${testStartTime}`

      await test.step('Given: 已存在一个已分配的套餐', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        // 创建并分配套餐
        await createBillingPlan(page, {
          planName,
          title: 'Assign Status Test Plan',
          price: '1000',
          type: 'monthly',
        })

        // 分配套餐
        await openAssignPlanDialog(page, planName)
        await page.getByTestId(/^plan-assignment-checkbox-/).first().check()
        await page.getByTestId('plan-assignment-submit-button').click()
        await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 5000 })
      })

      await test.step('When: 查看分配状态', async () => {
        await openAssignPlanDialog(page, planName)
      })

      await test.step('Then: 验证已分配的 Client App 被选中', async () => {
        await expect(page.getByTestId(/^plan-assignment-checkbox-/).first()).toBeChecked()
        demoLogger.testCode.log('[Test] ✓ Assigned app is checked')
      })
    })

    test('should remove plan assignment', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planName = `remove-assign-test-${testStartTime}`

      await test.step('Given: 已存在一个已分配的套餐', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        // 创建并分配套餐
        await createBillingPlan(page, {
          planName,
          title: 'Remove Assign Test Plan',
          price: '1000',
          type: 'monthly',
        })

        // 分配套餐
        await openAssignPlanDialog(page, planName)
        await page.getByTestId(/^plan-assignment-checkbox-/).first().check()
        await page.getByTestId('plan-assignment-submit-button').click()
        await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 5000 })
      })

      await test.step('When: 重新打开分配对话框', async () => {
        await openAssignPlanDialog(page, planName)
      })

      await test.step('And: 取消选中', async () => {
        await page.getByTestId(/^plan-assignment-checkbox-/).first().uncheck()
      })

      await test.step('And: 点击 "Assign Plan" 按钮', async () => {
        await page.getByTestId('plan-assignment-submit-button').click()
      })

      await test.step('Then: 验证分配移除成功', async () => {
        await expect(page.getByRole('dialog')).not.toBeVisible()
        demoLogger.testCode.log('[Test] ✓ Plan assignment removed successfully')
      })
    })
  })
})
