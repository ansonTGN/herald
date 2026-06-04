import { m } from '@/paraglide/messages'

export const USER_STATUS = {
  WAIT_VERIFIED: 0,
  NORMAL: 1,
  FORBIDDEN: 2,
  INVALID: 3,
} as const

export type UserStatus = (typeof USER_STATUS)[keyof typeof USER_STATUS]

export function getUserStatusLabel(status: number): string {
  switch (status) {
    case USER_STATUS.WAIT_VERIFIED:
      return m['user_status.wait_verified']()
    case USER_STATUS.NORMAL:
      return m['user_status.normal']()
    case USER_STATUS.FORBIDDEN:
      return m['user_status.forbidden']()
    case USER_STATUS.INVALID:
      return m['user_status.invalid']()
    default:
      return String(status)
  }
}

export function getUserStatusOptions() {
  return [
    { label: m['user_status.all'](), value: 'all' },
    { label: m['user_status.wait_verified'](), value: String(USER_STATUS.WAIT_VERIFIED) },
    { label: m['user_status.normal'](), value: String(USER_STATUS.NORMAL) },
    { label: m['user_status.forbidden'](), value: String(USER_STATUS.FORBIDDEN) },
    { label: m['user_status.invalid'](), value: String(USER_STATUS.INVALID) },
  ]
}

export const USER_STATUS_COLORS: Record<number, string> = {
  [USER_STATUS.WAIT_VERIFIED]: 'bg-yellow-100 text-yellow-800',
  [USER_STATUS.NORMAL]: 'bg-green-100 text-green-800',
  [USER_STATUS.FORBIDDEN]: 'bg-red-100 text-red-800',
  [USER_STATUS.INVALID]: 'bg-gray-100 text-gray-800',
}
