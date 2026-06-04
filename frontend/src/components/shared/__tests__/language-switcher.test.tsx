import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LanguageSwitcher } from '../language-switcher'
import { LocaleProvider } from '../locale-provider'

/**
 * Check whether a className contains the exact `bg-sidebar-accent` token
 * (not `bg-sidebar-accent/50` or other suffixed variants).
 */
function hasActiveBg(className: string): boolean {
  // Split by spaces and check for an exact match
  return className.split(' ').some((c) => c === 'bg-sidebar-accent')
}

describe('LanguageSwitcher', () => {
  beforeEach(() => {
    localStorage.clear()
    // Default to English so tests start from a predictable state
    localStorage.setItem('herald-locale', 'en')
  })

  /**
   * Render the LanguageSwitcher inside LocaleProvider.
   * We set up localStorage before rendering so the provider resolves
   * a deterministic initial locale.
   */
  function renderSwitcher() {
    return render(
      <LocaleProvider>
        <LanguageSwitcher />
      </LocaleProvider>
    )
  }

  describe('rendering both locale buttons', () => {
    it('renders the EN button with the correct testid', () => {
      renderSwitcher()
      expect(screen.getByTestId('language-switcher-en')).toBeInTheDocument()
    })

    it('renders the zh-CN button with the correct testid', () => {
      renderSwitcher()
      expect(screen.getByTestId('language-switcher-zh')).toBeInTheDocument()
    })

    it('displays EN label on the English button', () => {
      renderSwitcher()
      expect(screen.getByTestId('language-switcher-en')).toHaveTextContent('EN')
    })

    it('displays Chinese label on the zh-CN button', () => {
      renderSwitcher()
      expect(screen.getByTestId('language-switcher-zh')).toHaveTextContent('中文')
    })
  })

  describe('switching locale', () => {
    it('clicking the zh-CN button updates localStorage to zh-CN', async () => {
      const user = userEvent.setup()
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher-zh'))

      expect(localStorage.getItem('herald-locale')).toBe('zh-CN')
    })

    it('clicking the EN button updates localStorage to en', async () => {
      // Start in zh-CN
      localStorage.setItem('herald-locale', 'zh-CN')
      const user = userEvent.setup()
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher-en'))

      expect(localStorage.getItem('herald-locale')).toBe('en')
    })

    it('clicking EN then zh-CN leaves localStorage as zh-CN', async () => {
      const user = userEvent.setup()
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher-zh'))

      expect(localStorage.getItem('herald-locale')).toBe('zh-CN')
    })
  })

  describe('active locale indicator', () => {
    it('EN button has active styling when locale is en', () => {
      renderSwitcher()

      const enButton = screen.getByTestId('language-switcher-en')
      // Active state applies bg-sidebar-accent class (not the hover variant)
      expect(hasActiveBg(enButton.className)).toBe(true)
    })

    it('zh-CN button has inactive styling when locale is en', () => {
      renderSwitcher()

      const zhButton = screen.getByTestId('language-switcher-zh')
      // Inactive: only has hover:bg-sidebar-accent/50, not the exact active class
      expect(hasActiveBg(zhButton.className)).toBe(false)
    })

    it('after switching to zh-CN, zh button gains active styling', async () => {
      const user = userEvent.setup()
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher-zh'))

      const zhButton = screen.getByTestId('language-switcher-zh')
      expect(hasActiveBg(zhButton.className)).toBe(true)
    })

    it('after switching to zh-CN, EN button loses active styling', async () => {
      const user = userEvent.setup()
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher-zh'))

      const enButton = screen.getByTestId('language-switcher-en')
      expect(hasActiveBg(enButton.className)).toBe(false)
    })

    it('when starting in zh-CN, zh button has active styling', () => {
      localStorage.setItem('herald-locale', 'zh-CN')
      renderSwitcher()

      const zhButton = screen.getByTestId('language-switcher-zh')
      expect(hasActiveBg(zhButton.className)).toBe(true)
    })
  })
})
