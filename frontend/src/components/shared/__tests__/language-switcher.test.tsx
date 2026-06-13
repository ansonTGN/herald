import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LanguageSwitcher } from '../language-switcher'
import { LocaleProvider } from '../locale-provider'

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

  describe('trigger reflects the active locale', () => {
    it('shows the English label when locale is en', () => {
      renderSwitcher()
      expect(screen.getByTestId('language-switcher')).toHaveTextContent('English')
    })

    it('shows the Chinese label when locale is zh-CN', () => {
      localStorage.setItem('herald-locale', 'zh-CN')
      renderSwitcher()
      expect(screen.getByTestId('language-switcher')).toHaveTextContent('中文')
    })
  })

  describe('selecting a locale persists it', () => {
    it('selecting Chinese updates localStorage to zh-CN', async () => {
      const user = userEvent.setup({ delay: null })
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher'))
      const zhOption = await screen.findByRole('option', { name: '中文' })
      await user.click(zhOption)

      expect(localStorage.getItem('herald-locale')).toBe('zh-CN')
    })

    it('selecting English updates localStorage to en', async () => {
      // Start in zh-CN
      localStorage.setItem('herald-locale', 'zh-CN')
      const user = userEvent.setup({ delay: null })
      renderSwitcher()

      await user.click(screen.getByTestId('language-switcher'))
      const enOption = await screen.findByRole('option', { name: 'English' })
      await user.click(enOption)

      expect(localStorage.getItem('herald-locale')).toBe('en')
    })
  })
})
