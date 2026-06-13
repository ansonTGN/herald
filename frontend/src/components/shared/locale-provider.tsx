import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react'
import { setLocale, baseLocale, locales, type Locale } from '@/paraglide/runtime'

type LocaleContextValue = {
  locale: Locale
  switchLocale: (newLocale: Locale) => void
}

const LocaleContext = createContext<LocaleContextValue | null>(null)

/**
 * Detect the user's preferred locale from browser settings.
 * - zh prefix -> 'zh-CN'
 * - everything else -> 'en' (baseLocale)
 */
function detectBrowserLocale(): string {
  const browserLang = navigator.language
  if (browserLang.startsWith('zh')) {
    return 'zh-CN'
  }
  return baseLocale
}

/**
 * Resolve the initial locale: localStorage > browser detection > baseLocale.
 * Returns a locale that is in the project's supported locales list.
 */
function resolveInitialLocale(): Locale {
  const stored = localStorage.getItem('herald-locale')
  if (stored && locales.includes(stored as Locale)) {
    return stored as Locale
  }
  const detected = detectBrowserLocale()
  if (locales.includes(detected as Locale)) {
    return detected as Locale
  }
  return baseLocale
}

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocalState] = useState<Locale>(resolveInitialLocale)

  // Initialize Paraglide runtime on mount
  useEffect(() => {
    setLocale(locale, { reload: false })
  }, [locale])

  const switchLocale = useCallback(
    (newLocale: Locale) => {
      if (!locales.includes(newLocale)) return
      if (newLocale === locale) return
      // Reload the page so every component re-evaluates its `m.*` messages in the
      // new locale. Without the reload, only consumers of this context re-render;
      // the ~140 components that call `m.*` directly would keep the old language.
      setLocale(newLocale)
      setLocalState(newLocale)
    },
    [locale]
  )

  return (
    <LocaleContext.Provider value={{ locale, switchLocale }}>{children}</LocaleContext.Provider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useLocale(): LocaleContextValue {
  const ctx = useContext(LocaleContext)
  if (!ctx) {
    throw new Error('useLocale must be used within a LocaleProvider')
  }
  return ctx
}
