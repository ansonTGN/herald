export type PasswordStrengthLevel = 'weak' | 'fair' | 'good' | 'strong'

export interface PasswordStrength {
  score: number
  level: PasswordStrengthLevel
  color: 'red' | 'orange' | 'yellow' | 'green'
  /**
   * Unmet requirements, as keys into the `auth.register.password_strength.*`
   * i18n namespace. `{length}` is parameterized for the min-length message.
   */
  unmet: Array<{ key: PasswordStrengthMessageKey; length?: number }>
}

export type PasswordStrengthMessageKey =
  | 'min_length'
  | 'require_uppercase'
  | 'require_lowercase'
  | 'require_number'
  | 'require_special_char'

export interface PasswordConfig {
  minLength: number
  requireUppercase: boolean
  requireLowercase: boolean
  requireNumber: boolean
  requireSpecialChar: boolean
}

/**
 * Default password rules the strength meter advertises. Shared by the signup
 * and register forms so they stay in sync. The backend does not yet expose a
 * per-realm password policy (its public-config endpoint returns only
 * registration-enabled / email-verification flags), so this is the single
 * source of truth for now.
 */
export const DEFAULT_PASSWORD_CONFIG: PasswordConfig = {
  minLength: 8,
  requireUppercase: true,
  requireLowercase: true,
  requireNumber: true,
  requireSpecialChar: true,
}

type PasswordLevel = {
  level: PasswordStrengthLevel
  color: PasswordStrength['color']
}

const PASSWORD_LEVELS: PasswordLevel[] = [
  { level: 'weak', color: 'red' },
  { level: 'fair', color: 'orange' },
  { level: 'good', color: 'yellow' },
  { level: 'strong', color: 'green' },
]

function checkRequirement(password: string, regex: RegExp, required: boolean): boolean {
  if (!required) return true
  return regex.test(password)
}

export function calculatePasswordStrength(
  password: string,
  config: PasswordConfig
): PasswordStrength {
  let score = 0
  const unmet: PasswordStrength['unmet'] = []

  if (password.length >= config.minLength) {
    score += 1
  } else {
    unmet.push({ key: 'min_length', length: config.minLength })
  }

  const requirements = [
    {
      check: config.requireUppercase && !checkRequirement(password, /[A-Z]/, true),
      key: 'require_uppercase' as const,
    },
    {
      check: config.requireLowercase && !checkRequirement(password, /[a-z]/, true),
      key: 'require_lowercase' as const,
    },
    {
      check: config.requireNumber && !checkRequirement(password, /[0-9]/, true),
      key: 'require_number' as const,
    },
    {
      check: config.requireSpecialChar && !checkRequirement(password, /[^A-Za-z0-9]/, true),
      key: 'require_special_char' as const,
    },
  ]

  requirements.forEach((req) => {
    if (!req.check) {
      score += 1
    } else {
      unmet.push({ key: req.key })
    }
  })

  const levelIndex = Math.min(Math.floor(score / 1.25), 3)
  const level = PASSWORD_LEVELS[levelIndex]

  return {
    score,
    level: level.level,
    color: level.color,
    unmet,
  }
}
