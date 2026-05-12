export interface PasswordStrength {
  score: number
  label: 'Weak' | 'Fair' | 'Good' | 'Strong'
  color: 'red' | 'orange' | 'yellow' | 'green'
  suggestions: string[]
}

export interface PasswordConfig {
  minLength: number
  requireUppercase: boolean
  requireLowercase: boolean
  requireNumber: boolean
  requireSpecialChar: boolean
}

type PasswordLevel = { label: PasswordStrength['label']; color: PasswordStrength['color'] }

const PASSWORD_LEVELS: PasswordLevel[] = [
  { label: 'Weak', color: 'red' },
  { label: 'Fair', color: 'orange' },
  { label: 'Good', color: 'yellow' },
  { label: 'Strong', color: 'green' },
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
  const suggestions: string[] = []

  if (password.length >= config.minLength) {
    score += 1
  } else {
    suggestions.push(`Password must be at least ${config.minLength} characters`)
  }

  const requirements = [
    {
      check: config.requireUppercase && !checkRequirement(password, /[A-Z]/, true),
      message: 'Password must contain uppercase letters',
    },
    {
      check: config.requireLowercase && !checkRequirement(password, /[a-z]/, true),
      message: 'Password must contain lowercase letters',
    },
    {
      check: config.requireNumber && !checkRequirement(password, /[0-9]/, true),
      message: 'Password must contain numbers',
    },
    {
      check: config.requireSpecialChar && !checkRequirement(password, /[^A-Za-z0-9]/, true),
      message: 'Password must contain special characters',
    },
  ]

  requirements.forEach((req) => {
    if (!req.check) {
      score += 1
    } else {
      suggestions.push(req.message)
    }
  })

  const levelIndex = Math.min(Math.floor(score / 1.25), 3)
  const level = PASSWORD_LEVELS[levelIndex]

  return {
    score,
    label: level.label,
    color: level.color,
    suggestions,
  }
}
