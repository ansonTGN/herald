/**
 * WCAG contrast helpers for the white-label accent color field.
 *
 * Pure functions only — no DOM access, no React. Used by the white-label
 * config form to surface a WCAG AA warning when the configured accent color
 * has insufficient contrast against its (typically white) foreground. The
 * warning never blocks save/publish (per PRD: warn, don't intercept).
 */

/**
 * WCAG AA minimum contrast ratio for normal-sized text. The form shows
 * `white-label-accent-warning` when the computed ratio is strictly below
 * this threshold.
 */
export const WCAG_AA_MIN_CONTRAST = 4.5

/**
 * Parses a hex color (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`) into its
 * 8-bit RGB channels. Short forms (`#rgb`/`#rgba`) are expanded by doubling
 * each digit, matching the CSS hex color shorthand rules. Returns `null`
 * when the input cannot be parsed so callers can distinguish "no value"
 * from a valid color.
 */
function parseHexChannels(hexColor: string): { r: number; g: number; b: number } | null {
  const trimmed = hexColor.trim()
  if (!trimmed.startsWith('#')) return null

  const body = trimmed.slice(1)
  // Only accept the documented CSS hex lengths. Anything else (e.g. `#ggg`)
  // is rejected by the regex so we never feed `NaN` into the channel math.
  if (!/^([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(body)) {
    return null
  }

  let r: number
  let g: number
  let b: number

  if (body.length <= 4) {
    // Expand shorthand by doubling each hex digit (#rgb -> #rrggbb, #rgba -> #rrggbbaa)
    r = parseInt(body[0] + body[0], 16)
    g = parseInt(body[1] + body[1], 16)
    b = parseInt(body[2] + body[2], 16)
  } else {
    r = parseInt(body.slice(0, 2), 16)
    g = parseInt(body.slice(2, 4), 16)
    b = parseInt(body.slice(4, 6), 16)
  }

  return { r, g, b }
}

/**
 * Converts an sRGB channel value (0-255) to its relative luminance using the
 * WCAG 2.x piecewise transform. Returns a value in the closed interval [0, 1].
 */
function channelLuminance(channel: number): number {
  const c = channel / 255
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
}

/**
 * Relative luminance of an sRGB color per WCAG 2.x. Combines the per-channel
 * luminances with the sRGB→luminance weighting.
 */
function relativeLuminance(channels: { r: number; g: number; b: number }): number {
  const r = channelLuminance(channels.r)
  const g = channelLuminance(channels.g)
  const b = channelLuminance(channels.b)
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/**
 * Computes the WCAG contrast ratio between two colors as
 * `(L1 + 0.05) / (L2 + 0.05)` where `L1` is the lighter luminance and `L2`
 * the darker. Range is `1.0` (identical) to `21.0` (black vs white).
 */
function contrastBetween(
  a: { r: number; g: number; b: number },
  b: { r: number; g: number; b: number }
): number {
  const la = relativeLuminance(a)
  const lb = relativeLuminance(b)
  const lighter = Math.max(la, lb)
  const darker = Math.min(la, lb)
  return (lighter + 0.05) / (darker + 0.05)
}

/**
 * Computes the WCAG contrast ratio between a hex color (the accent / background)
 * and a foreground hex color (default white, the typical accent-on-white text).
 *
 * Supports `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` for both inputs. Returns
 * `NaN` when either color cannot be parsed so the caller can distinguish
 * "invalid input" from a valid (but low) ratio. The alpha channel, when
 * present, is ignored (WCAG contrast is defined on opaque colors).
 *
 * @example
 *   getContrastRatio('#000000')        // 21 (black vs white)
 *   getContrastRatio('#ffffff')        // 1  (white vs white)
 *   getContrastRatio('#777777')        // ~4.48 (just below AA)
 *   getContrastRatio('not-a-color')    // NaN
 */
export function getContrastRatio(hexColor: string, foreground = '#ffffff'): number {
  const bg = parseHexChannels(hexColor)
  const fg = parseHexChannels(foreground)
  if (!bg || !fg) return NaN
  return contrastBetween(bg, fg)
}
