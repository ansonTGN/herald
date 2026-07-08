import { describe, it, expect } from 'vitest'
import { getContrastRatio, WCAG_AA_MIN_CONTRAST } from '../white-label-contrast'

/**
 * Verified reference values produced by `getContrastRatio` against the default
 * white foreground. Each value was read from the actual implementation (sRGB
 * luminance + `(L1+0.05)/(L2+0.05)`) and pinned, not estimated, so a future
 * algorithm change does not silently shift the white-label warning behavior.
 *
 * - white-vs-white → 1.0 (minimum possible ratio, identical colors)
 * - black-vs-white → 21.0 (maximum possible ratio)
 * - yellow-vs-white (#ffff00) → ~1.074 (well below 4.5 → WCAG AA fail)
 * - yellow-vs-black (#ffff00 over #000000) → ~19.556 (high contrast)
 */
const WHITE_VS_WHITE = 1.0
const BLACK_VS_WHITE = 21.0
const YELLOW_VS_WHITE = 1.0738392309265699
const YELLOW_VS_BLACK = 19.555999999999997

describe('getContrastRatio', () => {
  describe('returns a high ratio for white/light backgrounds vs white foreground', () => {
    // White on white is the minimum possible contrast (identical colors → 1.0).
    // The form uses this to mean "no usable accent text contrast"; a value of
    // exactly 1.0 must never trip the `< 4.5` warning by accident of rounding.
    it.each([
      ['#fff (short form)', '#fff'],
      ['#ffffff (long form)', '#ffffff'],
    ])('returns the minimum ratio 1.0 for %s against white', (_label, hex) => {
      expect(getContrastRatio(hex)).toBe(WHITE_VS_WHITE)
    })
  })

  describe('returns the maximum ratio for black backgrounds vs white foreground', () => {
    // Black on white is the maximum WCAG ratio (21:1). The form uses this as the
    // canonical "passes AA with margin" accent color.
    it.each([
      ['#000 (short form)', '#000'],
      ['#000000 (long form)', '#000000'],
    ])('returns the maximum ratio 21.0 for %s against white', (_label, hex) => {
      expect(getContrastRatio(hex)).toBe(BLACK_VS_WHITE)
    })
  })

  describe('surfaces WCAG AA failures for low-contrast accents', () => {
    // #ffff00 is the documented canary: a yellow accent on white looks bright
    // but scores far below AA. The form's warning state depends on this being
    // strictly below WCAG_AA_MIN_CONTRAST.
    it('returns a ratio below WCAG AA for #ffff00 against white', () => {
      const ratio = getContrastRatio('#ffff00')

      expect(ratio).toBe(YELLOW_VS_WHITE)
      expect(ratio).toBeLessThan(WCAG_AA_MIN_CONTRAST)
    })
  })

  describe('returns NaN for unparseable inputs', () => {
    // NaN is the contract for "cannot evaluate contrast": the form must NOT
    // show the WCAG warning in this case (it shows a format hint instead).
    // Pinned to Number.isNaN so `undefined`/other non-numbers do not slip past.
    it.each([
      ['empty string', ''],
      ['named color', 'red'],
      ['missing leading hash', 'ffffff'],
      ['non-hex digits', '#gggggg'],
      ['wrong length (#zz)', '#zz'],
      ['three-char non-hex', '#xyz'],
      ['arbitrary text', 'not-a-color'],
    ])('returns NaN for %s', (_label, input) => {
      expect(Number.isNaN(getContrastRatio(input))).toBe(true)
    })
  })

  describe('the optional foreground parameter changes the computed ratio', () => {
    // The default foreground is white; passing black flips the comparison axis.
    // Same background color, opposite end of the luminance range → reciprocal-
    // ish ratio. This proves the foreground argument is actually wired through.
    it('uses the supplied foreground instead of defaulting to white', () => {
      expect(getContrastRatio('#ffff00', '#000000')).toBe(YELLOW_VS_BLACK)
    })

    it('produces a symmetric ratio regardless of which color is background vs foreground', () => {
      // contrastBetween takes max/min of luminances, so swapping bg/fg must
      // yield the same ratio. Guards against accidental argument-order bugs.
      const yellowOverBlack = getContrastRatio('#ffff00', '#000000')
      const blackOverYellow = getContrastRatio('#000000', '#ffff00')

      expect(blackOverYellow).toBe(yellowOverBlack)
    })
  })

  describe('treats alpha-carrying hex as opaque (alpha is ignored)', () => {
    // WCAG contrast is only defined on opaque colors. The implementation parses
    // #rgba / #rrggbbaa but discards alpha, so an opaque color and its
    // fully-opaque 8-channel twin must agree.
    it.each([
      ['#rgba matches #rgb', '#ffff', '#fff'],
      ['#rrggbbaa matches #rrggbb', '#ffffffff', '#ffffff'],
    ])('ignores alpha channel: %s', (_label, alphaHex, opaqueHex) => {
      expect(getContrastRatio(alphaHex)).toBe(getContrastRatio(opaqueHex))
    })
  })

  describe('returns NaN when only the foreground is invalid', () => {
    // The form foreground is fixed to white in practice, but the helper still
    // must not silently compute a bogus ratio if a caller passes bad input.
    it.each([
      ['invalid foreground', '#000000', 'white'],
      ['invalid background and foreground', 'nope', 'also-nope'],
    ])('returns NaN for %s', (_label, bg, fg) => {
      expect(Number.isNaN(getContrastRatio(bg, fg))).toBe(true)
    })
  })

  describe('produces ratios within the documented [1, 21] range', () => {
    // A spot check across the input space; any value outside [1, 21] would
    // indicate a luminance or formula regression.
    it.each([
      ['#777777 (just below AA per jsdoc)', '#777777'],
      ['#2563eb (brand blue)', '#2563eb'],
      ['#000000', '#000000'],
      ['#ffffff', '#ffffff'],
    ])('keeps %s within [1, 21]', (_label, hex) => {
      const ratio = getContrastRatio(hex)
      expect(ratio).toBeGreaterThanOrEqual(1)
      expect(ratio).toBeLessThanOrEqual(21)
    })
  })
})

describe('WCAG_AA_MIN_CONTRAST', () => {
  // The form warns when the computed ratio is *strictly below* this constant.
  // Pin it so a future edit (e.g. AA Large Text 3.0) is an intentional change.
  it('is 4.5 (WCAG AA normal text)', () => {
    expect(WCAG_AA_MIN_CONTRAST).toBe(4.5)
  })
})
