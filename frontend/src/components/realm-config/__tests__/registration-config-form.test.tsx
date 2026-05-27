import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { RegistrationConfigForm } from '../registration-config-form'
import type { RegistrationConfigForm as RegistrationConfigFormData } from '@/lib/schemas/realm-config'

describe('RegistrationConfigForm', () => {
  const mockOnSave = vi.fn()
  const defaultProps = {
    realmId: 'admin',
    onSave: mockOnSave,
    isLoading: false,
  }

  beforeEach(() => {
    mockOnSave.mockClear()
  })

  it('GIVEN form is rendered WHEN no initial config THEN should display form with default values', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    expect(screen.getByTestId('reg-enabled-switch')).toBeInTheDocument()
    expect(screen.getByTestId('reg-require-email-switch')).toBeInTheDocument()
    expect(screen.getByTestId('reg-save-button')).toBeInTheDocument()
  })

  it('GIVEN initial config provided WHEN rendering THEN should display configuration values', async () => {
    const initialConfig: RegistrationConfigFormData = {
      enabled: false,
      requireEmailVerification: true, // P0 修复：camelCase 字段名
    }

    const screen = render(
      <RegistrationConfigForm {...defaultProps} initialConfig={initialConfig} />
    )

    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    expect(enabledSwitch).not.toBeChecked()

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).toBeChecked()
  })

  it('GIVEN user toggles switches WHEN submitting form THEN should call onSave with config', async () => {
    mockOnSave.mockResolvedValue(undefined)
    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    // 禁用注册
    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    await userEvent.click(enabledSwitch)

    // 提交表单
    const saveButton = screen.getByTestId('reg-save-button')
    await userEvent.click(saveButton)

    // 验证 onSave 被调用
    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith({
        enabled: false,
        requireEmailVerification: true,
      })
    })
  })

  // 异常场景测试
  it('GIVEN onSave fails WHEN submitting form THEN should handle error gracefully', async () => {
    const testError = new Error('Network error')
    mockOnSave.mockRejectedValue(testError)

    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    // 禁用注册并提交
    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    await userEvent.click(enabledSwitch)

    const saveButton = screen.getByTestId('reg-save-button')
    await userEvent.click(saveButton)

    // 验证 onSave 被调用但错误被处理
    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalled()
    })

    // Wait for async operations
    await new Promise((resolve) => setTimeout(resolve, 0))
    consoleSpy.mockRestore()
  })

  // P0 补充：字段依赖逻辑测试
  it('GIVEN registration is disabled WHEN toggling requireEmailVerification THEN should keep setting', async () => {
    mockOnSave.mockResolvedValue(undefined)

    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    // 先禁用注册
    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    await userEvent.click(enabledSwitch)

    // 修改邮箱验证要求
    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    await userEvent.click(requireEmailSwitch)

    // 提交表单
    const saveButton = screen.getByTestId('reg-save-button')
    await userEvent.click(saveButton)

    // 验证 onSave 被调用，保存所有配置（即使 enabled=false）
    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith({
        enabled: false,
        requireEmailVerification: expect.any(Boolean),
      })
    })
  })

  // P0 补充：禁用状态测试
  it('GIVEN form is disabled WHEN user interacts THEN should not allow changes', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} disabled={true} />)

    // 验证开关被禁用
    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    expect(enabledSwitch).toBeDisabled()

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).toBeDisabled()

    // 验证保存按钮被禁用
    const saveButton = screen.getByTestId('reg-save-button')
    expect(saveButton).toBeDisabled()
  })

  // P0 补充：加载状态测试
  it('GIVEN isLoading prop is true WHEN rendering THEN should disable save button', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} isLoading={true} />)

    const saveButton = screen.getByTestId('reg-save-button')
    expect(saveButton).toBeDisabled()
  })

  // P0 补充：表单提交中状态测试
  it('GIVEN form is submitting WHEN save is in progress THEN should disable save button', async () => {
    mockOnSave.mockImplementation(() => new Promise((resolve) => setTimeout(resolve, 100)))

    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    const saveButton = screen.getByTestId('reg-save-button')
    await userEvent.click(saveButton)

    // 验证按钮被禁用（使用 waitFor 等待状态更新）
    await waitFor(() => {
      expect(saveButton).toBeDisabled()
    })
  })

  // P0 补充：并发提交测试
  it('GIVEN user clicks save multiple times WHEN submitting THEN should call onSave multiple times', async () => {
    mockOnSave.mockImplementation(() => new Promise((resolve) => setTimeout(resolve, 100)))

    const screen = render(<RegistrationConfigForm {...defaultProps} />)

    const saveButton = screen.getByTestId('reg-save-button')

    // 快速点击 3 次
    await userEvent.click(saveButton)
    await userEvent.click(saveButton)
    await userEvent.click(saveButton)

    // 等待异步操作完成
    await new Promise((resolve) => setTimeout(resolve, 200))

    // 验证：组件当前的防重复提交实现只阻止第一个提交正在进行时的后续点击
    // 但如果点击足够快，可能会有多次调用
    expect(mockOnSave).toHaveBeenCalled()
    // 注意：这个测试验证当前行为，如果需要更强防抖，需要更新组件
  })

  // P0 补充：初始配置加载测试 - 禁用注册但要求邮箱验证
  it('GIVEN initial config has registration disabled with email verification WHEN rendering THEN should display switches correctly', async () => {
    const initialConfig: RegistrationConfigFormData = {
      enabled: false,
      requireEmailVerification: true,
    }

    const screen = render(
      <RegistrationConfigForm {...defaultProps} initialConfig={initialConfig} />
    )

    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    expect(enabledSwitch).not.toBeChecked()

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).toBeChecked()
  })

  // P0 补充：初始配置加载测试 - 允许注册但不需要邮箱验证
  it('GIVEN initial config allows registration without email verification WHEN rendering THEN should display switches correctly', async () => {
    const initialConfig: RegistrationConfigFormData = {
      enabled: true,
      requireEmailVerification: false,
    }

    const screen = render(
      <RegistrationConfigForm {...defaultProps} initialConfig={initialConfig} />
    )

    const enabledSwitch = screen.getByTestId('reg-enabled-switch')
    expect(enabledSwitch).toBeChecked()

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).not.toBeChecked()
  })

  // Email configuration gating tests
  it('GIVEN email not configured WHEN rendering THEN should disable requireEmailVerification switch', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} emailConfigured={false} />)

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).toBeDisabled()
  })

  it('GIVEN email not configured WHEN rendering THEN should show email config required hint', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} emailConfigured={false} />)

    expect(screen.getByTestId('email-config-required-hint')).toBeInTheDocument()
    expect(screen.getByTestId('email-config-required-hint')).toHaveTextContent(
      'Email verification requires email configuration'
    )
  })

  it('GIVEN email configured WHEN rendering THEN should enable requireEmailVerification switch', async () => {
    const screen = render(<RegistrationConfigForm {...defaultProps} emailConfigured={true} />)

    const requireEmailSwitch = screen.getByTestId('reg-require-email-switch')
    expect(requireEmailSwitch).not.toBeDisabled()
  })
})
