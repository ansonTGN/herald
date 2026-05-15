import { describe, it, expect, vi, afterEach } from 'vitest'
import { render } from '@testing-library/react'
import { PermissionTag } from '../permission-tag'

describe('PermissionTag', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN permission action is manage WHEN rendering tag THEN should show destructive variant', async () => {
    const screen = render(<PermissionTag permission="users.manage" />)
    const tag = screen.getByTestId('permission-tag-users-manage')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveClass(/bg-destructive/)
  })

  it('GIVEN permission action is view WHEN rendering tag THEN should show default variant', async () => {
    const screen = render(<PermissionTag permission="users.view" />)
    const tag = screen.getByTestId('permission-tag-users-view')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveClass(/bg-primary/)
  })

  it('GIVEN permission action is delete WHEN rendering tag THEN should show destructive variant', async () => {
    const screen = render(<PermissionTag permission="users.delete" />)
    const tag = screen.getByTestId('permission-tag-users-delete')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveClass(/bg-destructive/)
  })

  it('GIVEN permission action is list WHEN rendering tag THEN should show default variant', async () => {
    const screen = render(<PermissionTag permission="users.list" />)
    const tag = screen.getByTestId('permission-tag-users-list')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveClass(/bg-primary/)
  })
})
