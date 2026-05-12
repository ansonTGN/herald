export const USER_STATUS = {
  WAIT_VERIFIED: 0,
  NORMAL: 1,
  FORBIDDEN: 2,
  INVALID: 3,
} as const

export type UserStatus = (typeof USER_STATUS)[keyof typeof USER_STATUS]

export const USER_STATUS_LABELS: Record<number, string> = {
  [USER_STATUS.WAIT_VERIFIED]: 'Wait Verified',
  [USER_STATUS.NORMAL]: 'Normal',
  [USER_STATUS.FORBIDDEN]: 'Forbidden',
  [USER_STATUS.INVALID]: 'Invalid',
}

export const USER_STATUS_COLORS: Record<number, string> = {
  [USER_STATUS.WAIT_VERIFIED]: 'bg-yellow-100 text-yellow-800',
  [USER_STATUS.NORMAL]: 'bg-green-100 text-green-800',
  [USER_STATUS.FORBIDDEN]: 'bg-red-100 text-red-800',
  [USER_STATUS.INVALID]: 'bg-gray-100 text-gray-800',
}

export const USER_STATUS_OPTIONS = [
  { label: 'All', value: 'all' },
  { label: 'Wait Verified', value: String(USER_STATUS.WAIT_VERIFIED) },
  { label: 'Normal', value: String(USER_STATUS.NORMAL) },
  { label: 'Forbidden', value: String(USER_STATUS.FORBIDDEN) },
  { label: 'Invalid', value: String(USER_STATUS.INVALID) },
] as const
