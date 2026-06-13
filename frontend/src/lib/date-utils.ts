/**
 * Convert a date string or ISO string to a date input value (YYYY-MM-DD)
 * @param value - Optional date string
 * @returns Date input value string or empty string
 */
export function toDateInputValue(value?: string): string {
  if (!value) {
    return ''
  }
  return value.includes('T') ? value.split('T')[0] : value
}

/**
 * Convert a date string to a UTC ISO date range boundary
 * @param value - Date string in YYYY-MM-DD format
 * @param boundary - 'start' for T00:00:00.000Z or 'end' for T23:59:59.999Z
 * @returns ISO date string with UTC time boundary
 */
export function toUtcDateRangeBoundary(value: string, boundary: 'start' | 'end'): string {
  const suffix = boundary === 'start' ? 'T00:00:00.000Z' : 'T23:59:59.999Z'
  return new Date(`${value}${suffix}`).toISOString()
}

/**
 * Format a date string for display
 * @param dateString - Optional date string or ISO string
 * @returns Formatted date string or '-' if no date
 */
export function formatDate(dateString: string | null | undefined): string {
  if (!dateString) return '-'
  return new Date(dateString).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Compact fixed-width date-time with explicit local UTC offset: YYYY-MM-DD HH:mm UTC+H[:MM].
 * Local wall-clock time + offset so the same instant is unambiguous across viewers and
 * reconcilable with the backend's UTC (DateTime<Utc>) storage. No seconds.
 * Deterministic across locales — built from date parts, not toLocaleString.
 */
export function formatDateTimeShort(dateString: string): string {
  const d = new Date(dateString)
  const pad = (n: number) => String(n).padStart(2, '0')
  // getTimezoneOffset is in minutes, positive when local is BEHIND UTC (sign inverted).
  const off = d.getTimezoneOffset()
  const sign = off <= 0 ? '+' : '-'
  const abs = Math.abs(off)
  const h = Math.floor(abs / 60)
  const mm = abs % 60
  const offset = mm === 0 ? `UTC${sign}${h}` : `UTC${sign}${h}:${pad(mm)}`
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())} ${offset}`
}
