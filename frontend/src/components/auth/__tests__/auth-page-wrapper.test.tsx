import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { AuthPageWrapper } from '../auth-page-wrapper'
import type { PublicWhiteLabelConfig } from '@/lib/api-generated/types.gen'

/**
 * jsdom's built-in `Image()` never fires `load`/`error`, which would leave the
 * background preload pending forever. We install a minimal fake that records the
 * handlers and lets each test drive them (or auto-fails) so we can assert both
 * the success and fallback paths deterministically.
 */
type FakeImage = {
  onload: (() => void) | null
  onerror: (() => void) | null
  src: string
}

type ImageMode = 'success' | 'error'

function installFakeImage(mode: ImageMode) {
  const instances: FakeImage[] = []
  // A real constructor (not an arrow fn) so `new Image()` works; vitest can spy
  // on its calls via `ctor.mock`.
  const Ctor = vi.fn(function (this: FakeImage) {
    let backingSrc = ''
    const img: FakeImage = { onload: null, onerror: null, src: '' }
    Object.defineProperty(this, 'onload', {
      configurable: true,
      get: () => img.onload,
      set: (v: (() => void) | null) => {
        img.onload = v
      },
    })
    Object.defineProperty(this, 'onerror', {
      configurable: true,
      get: () => img.onerror,
      set: (v: (() => void) | null) => {
        img.onerror = v
      },
    })
    Object.defineProperty(this, 'src', {
      configurable: true,
      get() {
        return backingSrc
      },
      set(value: string) {
        backingSrc = value
        // Mimic the native async decode + dispatch on the next macrotask.
        setTimeout(() => {
          if (mode === 'error') img.onerror?.()
          else img.onload?.()
        }, 0)
      },
    })
    instances.push(this)
  })
  vi.stubGlobal('Image', Ctor)
  return { ctor: Ctor, instances }
}

const DEFAULT_BG_CLASS = 'bg-gradient-to-b'

function rootEl(container: HTMLElement): HTMLElement {
  // The root is the wrapper div that carries the default gradient class.
  const el = container.querySelector(`.${DEFAULT_BG_CLASS}`)
  if (!el) throw new Error('AuthPageWrapper root element not found')
  return el as HTMLElement
}

describe('AuthPageWrapper', () => {
  let originalImage: typeof globalThis.Image | undefined

  beforeEach(() => {
    originalImage = globalThis.Image
  })

  afterEach(() => {
    if (originalImage) {
      vi.stubGlobal('Image', originalImage)
    }
    vi.unstubAllGlobals()
  })

  describe('logo', () => {
    it('GIVEN logoUrl present WHEN rendering THEN shows the logo image, not the text fallback', async () => {
      const whiteLabel: PublicWhiteLabelConfig = { logoUrl: 'https://cdn.example.com/logo.svg' }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const logo = screen.getByTestId('auth-brand-logo')
      expect(logo).toBeInTheDocument()
      expect(logo).toHaveAttribute('src', 'https://cdn.example.com/logo.svg')
      expect(screen.queryByTestId('auth-brand-text')).not.toBeInTheDocument()
    })

    it('GIVEN no logoUrl WHEN rendering THEN shows the Herald text fallback', async () => {
      const screen = render(
        <AuthPageWrapper whiteLabel={{}}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const text = screen.getByTestId('auth-brand-text')
      expect(text).toBeInTheDocument()
      expect(text).toHaveTextContent('Herald')
      expect(screen.queryByTestId('auth-brand-logo')).not.toBeInTheDocument()
    })

    it('GIVEN logo fails to load WHEN onError fires THEN switches to the Herald text fallback', async () => {
      const whiteLabel: PublicWhiteLabelConfig = { logoUrl: 'https://broken.example.com/x.png' }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const logo = screen.getByTestId('auth-brand-logo')
      // React wires `onError` to the native error event; fire it like a broken load.
      fireEvent.error(logo)
      expect(screen.queryByTestId('auth-brand-logo')).not.toBeInTheDocument()
      const text = screen.getByTestId('auth-brand-text')
      expect(text).toBeInTheDocument()
      expect(text).toHaveTextContent('Herald')
    })
  })

  describe('accent color', () => {
    it('GIVEN a valid accentColor WHEN rendering THEN sets --primary and --ring on the root style, not className', async () => {
      const whiteLabel: PublicWhiteLabelConfig = { accentColor: '#2563eb' }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      const style = root.getAttribute('style') ?? ''
      // Assert presence of the CSS variable overrides (never on className).
      expect(style).toContain('--primary: #2563eb')
      expect(style).toContain('--ring: #2563eb')
      expect(root.className).not.toContain('#2563eb')
    })

    it('GIVEN no accentColor WHEN rendering THEN leaves the CSS variables unset', async () => {
      const screen = render(
        <AuthPageWrapper whiteLabel={{}}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      const style = root.getAttribute('style') ?? ''
      expect(style).not.toContain('--primary')
      expect(style).not.toContain('--ring')
    })
  })

  describe('footer', () => {
    it('GIVEN footerText present WHEN rendering THEN renders the footer', async () => {
      const screen = render(
        <AuthPageWrapper whiteLabel={{ footerText: 'Example Inc.' }}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const footer = screen.getByTestId('auth-brand-footer')
      expect(footer).toBeInTheDocument()
      expect(footer).toHaveTextContent('Example Inc.')
    })

    it('GIVEN no footerText WHEN rendering THEN does not render the footer', async () => {
      const screen = render(
        <AuthPageWrapper whiteLabel={{}}>
          <div>child</div>
        </AuthPageWrapper>
      )
      expect(screen.queryByTestId('auth-brand-footer')).not.toBeInTheDocument()
    })
  })

  describe('background', () => {
    it('GIVEN an image background that loads WHEN rendering THEN applies backgroundImage via style', async () => {
      installFakeImage('success')
      const whiteLabel: PublicWhiteLabelConfig = {
        background: { type: 'image', value: 'https://cdn.example.com/bg.jpg' },
      }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      await waitFor(() => {
        const style = root.getAttribute('style') ?? ''
        expect(style).toContain('background-image')
        expect(style).toContain('https://cdn.example.com/bg.jpg')
      })
    })

    it('GIVEN an image background that fails WHEN rendering THEN falls back to the default gradient (no backgroundImage)', async () => {
      installFakeImage('error')
      const whiteLabel: PublicWhiteLabelConfig = {
        background: { type: 'image', value: 'https://broken.example.com/bg.jpg' },
      }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      await waitFor(() => {
        const style = root.getAttribute('style') ?? ''
        expect(style).not.toContain('background-image')
      })
      // Default gradient class remains intact as the fallback.
      expect(root.className).toContain(DEFAULT_BG_CLASS)
    })

    it('GIVEN a valid gradient background WHEN rendering THEN applies the gradient via style', async () => {
      const whiteLabel: PublicWhiteLabelConfig = {
        background: { type: 'gradient', value: 'linear-gradient(to right, #1e3a8a, #2563eb)' },
      }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      const style = root.getAttribute('style') ?? ''
      // jsdom normalizes the hex stops to rgb(), so assert on the stable prefix.
      expect(style).toContain('background-image: linear-gradient(to right')
    })

    it('GIVEN an invalid gradient background WHEN rendering THEN falls back to the default gradient', async () => {
      const whiteLabel: PublicWhiteLabelConfig = {
        background: { type: 'gradient', value: 'url("https://evil.example.com/x.png")' },
      }
      const screen = render(
        <AuthPageWrapper whiteLabel={whiteLabel}>
          <div>child</div>
        </AuthPageWrapper>
      )
      const root = rootEl(screen.container)
      const style = root.getAttribute('style') ?? ''
      expect(style).not.toContain('background-image')
      expect(root.className).toContain(DEFAULT_BG_CLASS)
    })
  })

  describe('children', () => {
    it('GIVEN children WHEN rendering THEN renders them unchanged', async () => {
      const screen = render(
        <AuthPageWrapper whiteLabel={null}>
          <div data-testid="login-form">form</div>
        </AuthPageWrapper>
      )
      expect(screen.getByTestId('login-form')).toBeInTheDocument()
    })

    it('GIVEN no whiteLabel at all WHEN rendering THEN renders Herald text and keeps default gradient', async () => {
      const screen = render(<AuthPageWrapper>children</AuthPageWrapper>)
      expect(screen.getByTestId('auth-brand-text')).toHaveTextContent('Herald')
      const root = rootEl(screen.container)
      expect(root.className).toContain(DEFAULT_BG_CLASS)
    })
  })
})
