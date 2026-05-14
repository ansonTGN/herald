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

export function formatDateTime(dateString: string): string {
  return new Date(dateString).toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}
