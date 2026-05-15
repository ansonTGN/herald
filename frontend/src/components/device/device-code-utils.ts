const VALID_CHARS = new Set('BCDFGHJKMNPQRSTVWXYZ')

export function filterAndFormat(value: string): string {
  const upper = value.toUpperCase()
  const filtered = upper
    .split('')
    .filter((c) => VALID_CHARS.has(c))
    .slice(0, 8)
    .join('')
  if (filtered.length <= 4) return filtered
  return filtered.slice(0, 4) + '-' + filtered.slice(4)
}

export function toBackendCode(formatted: string): string {
  return formatted.replace(/-/g, '')
}

export function rawLength(formatted: string): number {
  return formatted.replace(/-/g, '').length
}
