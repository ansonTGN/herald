import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PermissionTag } from '../permission-tag'

describe('PermissionTag', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN permission is users.view WHEN rendering tag THEN should display permission name correctly', async () => {
    const screen = render(<PermissionTag permission="users.view" />)
    const tag = screen.getByTestId('permission-tag-users-view')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveTextContent('users.view')
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

  it('GIVEN permission action is unknown WHEN rendering tag THEN should show secondary variant', async () => {
    const screen = render(<PermissionTag permission="users.unknown" />)
    const tag = screen.getByTestId('permission-tag-users-unknown')
    expect(tag).toBeInTheDocument()
    expect(tag).toHaveClass(/bg-secondary/)
  })

  it('GIVEN showDescription is true WHEN rendering tag THEN should display resource in parentheses', async () => {
    const screen = render(<PermissionTag permission="users.view" showDescription={true} />)
    const tag = screen.getByTestId('permission-tag-users-view')
    expect(tag).toHaveTextContent(/(users)/)
  })

  it('GIVEN custom className is provided WHEN rendering THEN should merge classes', async () => {
    const screen = render(<PermissionTag permission="users.view" className="custom-class" />)
    const tag = screen.getByTestId('permission-tag-users-view')
    expect(tag).toHaveClass(/custom-class/)
  })
})
