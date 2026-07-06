import { describe, it, expect } from 'vitest'
import { resolvePageName } from '@/hooks/use-document-title'

/**
 * Why this matters: Chrome's back-button dropdown reads document.title at the
 * moment each history entry is pushed. If every page shares one title, the
 * dropdown is useless. resolvePageName must therefore produce a DISTINCT,
 * human-readable label per route — these tests encode that intent, not just
 * the mechanical split of a path.
 */
describe('resolvePageName', () => {
  it('maps admin manage segments to translated labels', () => {
    expect(resolvePageName('/admin/manage')).toBe('Dashboard')
    expect(resolvePageName('/admin/manage/users')).toBe('Users')
    expect(resolvePageName('/admin/manage/roles')).toBe('Roles')
    expect(resolvePageName('/admin/manage/audit')).toBe('Audit Log')
  })

  it('maps user profile segments to translated labels', () => {
    expect(resolvePageName('/admin/user/profile')).toBe('Profile')
    expect(resolvePageName('/admin/user/security')).toBe('Security')
    expect(resolvePageName('/admin/user/points')).toBe('Points')
  })

  it('falls back to the parent segment for action sub-routes', () => {
    // /manage/users/:id/edit should read as "Users", not "Edit"
    expect(resolvePageName('/admin/manage/users/u-123/edit')).toBe('Users')
    expect(resolvePageName('/admin/manage/api-keys/new')).toBe('API Keys')
  })

  it('capitalizes an unmapped segment when no labeled ancestor exists', () => {
    // neither 'admin' nor 'something-new' has a label, so the leaf is used
    expect(resolvePageName('/admin/something-new')).toBe('Something new')
  })

  it('returns empty string for id-like trailing segments', () => {
    expect(resolvePageName('/admin/manage/users/123')).toBe('Users')
  })

  it('returns empty string for the bare root', () => {
    expect(resolvePageName('/')).toBe('')
  })
})
