import {
  calculatePasswordStrength,
  type PasswordConfig,
  type PasswordStrength,
  type PasswordStrengthMessageKey,
} from '@/lib/password-strength'
import { m } from '@/paraglide/messages'

interface PasswordStrengthMeterProps {
  password: string
  config: PasswordConfig
}

const COLOR_CLASSES: Record<PasswordStrength['color'], string> = {
  red: 'bg-red-500',
  orange: 'bg-orange-500',
  yellow: 'bg-yellow-500',
  green: 'bg-green-500',
}

function translateUnmet(item: { key: PasswordStrengthMessageKey; length?: number }): string {
  const key = item.key
  return key === 'min_length'
    ? m['auth.register.password_strength.min_length']({ length: item.length ?? 0 })
    : m[`auth.register.password_strength.${key}`]()
}

export function PasswordStrengthMeter({ password, config }: PasswordStrengthMeterProps) {
  // An empty field is not a "violation" worth flagging — surface the meter and
  // its rules only once the user has typed something, so we don't dump all five
  // requirements on load.
  if (password.length === 0) {
    return null
  }

  const strength = calculatePasswordStrength(password, config)
  const colorClass = COLOR_CLASSES[strength.color]

  return (
    <div className="mt-2">
      <div className="flex items-center gap-2">
        <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${colorClass}`}
            style={{ width: `${(strength.score / 4) * 100}%` }}
          />
        </div>
        <span className="text-sm font-medium">
          {m[`auth.register.password_strength.${strength.level}`]()}
        </span>
      </div>

      {strength.unmet.length > 0 && (
        <ul className="mt-2 text-sm text-muted-foreground">
          {strength.unmet.map((item) => (
            <li key={item.key}>• {translateUnmet(item)}</li>
          ))}
        </ul>
      )}
    </div>
  )
}
