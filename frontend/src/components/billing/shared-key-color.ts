/**
 * Stable hue derivation for a shared entitlement key.
 *
 * Same `entitlementKey` always maps to the same hue so the admin can scan
 * which prices share a key across the master-detail view.
 *
 * IMPORTANT: the color is decorative only — tests and the Demo layer locate
 * elements via the `shared-key-chip-${entitlementKey}` testid, NEVER via the
 * computed color. Do not promote `className`/`hue` to a selector contract.
 */

/** FNV-1a 32-bit hash — small, dependency-free, stable across runs. */
function hashString(input: string): number {
  let hash = 0x811c9dc5
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i)
    hash = Math.imul(hash, 0x01000193)
  }
  return hash >>> 0
}

export interface SharedKeyColor {
  /** 0–360 hue value (HSL). */
  hue: number
  /**
   * Layout / neutral classes (e.g. `bg-muted-foreground` for an unconfigured
   * key). The dynamic hue is applied by the caller via inline `style` (Tailwind
   * can't bind dynamic hues without safelisting).
   */
  className: string
}

/**
 * Derive a stable `{ hue, className }` for an entitlement key.
 * Empty keys collapse to a neutral gray so an unconfigured row does not get
 * a misleading "shared" color.
 */
export function deriveSharedKeyColor(key: string): SharedKeyColor {
  if (!key) {
    return { hue: 0, className: 'bg-muted-foreground' }
  }
  const hue = hashString(key) % 360
  return {
    hue,
    // `style={{ backgroundColor: \`hsl(${hue} 70% 50%)\` }}` is applied by the
    // caller on the dot/band element; className is reserved for layout only.
    className: '',
  }
}
