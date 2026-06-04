import { useLocale } from '@/components/shared/locale-provider'

export function LanguageSwitcher() {
  const { locale, switchLocale } = useLocale()

  return (
    <div data-testid="language-switcher" className="flex gap-1">
      <button
        data-testid="language-switcher-en"
        onClick={() => switchLocale('en')}
        className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors ${
          locale === 'en'
            ? 'bg-sidebar-accent text-sidebar-foreground'
            : 'text-sidebar-foreground/50 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground/70'
        }`}
      >
        EN
      </button>
      <button
        data-testid="language-switcher-zh"
        onClick={() => switchLocale('zh-CN')}
        className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors ${
          locale === 'zh-CN'
            ? 'bg-sidebar-accent text-sidebar-foreground'
            : 'text-sidebar-foreground/50 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground/70'
        }`}
      >
        中文
      </button>
    </div>
  )
}
