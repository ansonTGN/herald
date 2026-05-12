import {
  calculatePasswordStrength,
  type PasswordConfig,
  type PasswordStrength,
} from '@/lib/password-strength'

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

export function PasswordStrengthMeter({ password, config }: PasswordStrengthMeterProps) {
  const strength = calculatePasswordStrength(password, config)
  const colorClass = COLOR_CLASSES[strength.color]

  return (
    <div className="mt-2">
      <div className="flex items-center gap-2">
        <div className="flex-1 h-2 bg-gray-200 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${colorClass}`}
            style={{ width: `${(strength.score / 4) * 100}%` }}
          />
        </div>
        <span className="text-sm font-medium">{strength.label}</span>
      </div>

      {strength.suggestions.length > 0 && (
        <ul className="mt-2 text-sm text-gray-600">
          {strength.suggestions.map((suggestion) => (
            <li key={suggestion}>• {suggestion}</li>
          ))}
        </ul>
      )}
    </div>
  )
}
