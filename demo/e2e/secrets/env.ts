/**
 * Type-safe environment variable reader for demo tests.
 *
 * Values come from `.env.demo` (loaded by playwright.config.ts via dotenv).
 * All fields are `string | undefined` -- callers must check before use.
 */

export const secrets = {
  github: {
    clientId: process.env.GITHUB_CLIENT_ID,
    clientSecret: process.env.GITHUB_CLIENT_SECRET,
  },
  creem: {
    apiKey: process.env.CREEM_API_KEY,
    webhookSecret: process.env.CREEM_WEBHOOK_SECRET,
    productId: process.env.CREEM_PRODUCT_ID,
    onetimeProductId: process.env.CREEM_ONETIME_PRODUCT_ID,
  },
  stripe: {
    publishableKey: process.env.STRIPE_PUBLISHABLE_KEY,
    secretKey: process.env.STRIPE_SECRET_KEY,
    webhookSecret: process.env.STRIPE_WEBHOOK_SECRET,
    productId: process.env.STRIPE_PRODUCT_ID,
    onetimeProductId: process.env.STRIPE_ONETIME_PRODUCT_ID,
  },
  ngrok: {
    authtoken: process.env.NGROK_AUTHTOKEN,
    domain: process.env.NGROK_DOMAIN,
  },
  qq: {
    smtpHost: process.env.QQ_SMTP_HOST,
    smtpPort: process.env.QQ_SMTP_PORT, // string, e.g. "465"
    smtpUsername: process.env.QQ_SMTP_USERNAME, // full mailbox, e.g. xxx@qq.com
    smtpPassword: process.env.QQ_SMTP_PASSWORD, // 16-char authorization code (not login password)
    smtpEncryption: process.env.QQ_SMTP_ENCRYPTION, // "ssl" | "starttls", optional, default ssl
    fromAddress: process.env.QQ_FROM_ADDRESS, // optional, defaults to smtpUsername
    testRecipient: process.env.QQ_TEST_RECIPIENT, // optional test email recipient
  },
} as const

// --- Predicate helpers ---

export function hasGitHubOAuth(): boolean {
  return !!(secrets.github.clientId && secrets.github.clientSecret)
}

export function hasCreemPayment(): boolean {
  return !!(secrets.creem.apiKey && secrets.creem.webhookSecret && secrets.creem.productId)
}

export function hasStripePayment(): boolean {
  return !!(
    secrets.stripe.publishableKey &&
    secrets.stripe.secretKey &&
    secrets.stripe.webhookSecret &&
    secrets.stripe.productId
  )
}

// --- Require helpers (throw with actionable message) ---

export function requireGitHubOAuth(): void {
  if (!hasGitHubOAuth()) {
    throw new Error(
      'GitHub OAuth credentials not configured. ' +
        'Set GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET in demo/.env.demo.',
    )
  }
}

export function requireCreemPayment(): void {
  if (!hasCreemPayment()) {
    throw new Error(
      'Creem payment credentials not configured. ' +
        'Set CREEM_API_KEY, CREEM_WEBHOOK_SECRET, and CREEM_PRODUCT_ID in demo/.env.demo.',
    )
  }
}

export function requireStripePayment(): void {
  if (!hasStripePayment()) {
    throw new Error(
      'Stripe payment credentials not configured. ' +
        'Set STRIPE_PUBLISHABLE_KEY, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, and STRIPE_PRODUCT_ID in demo/.env.demo.',
    )
  }
}

export function hasNgrok(): boolean {
  return !!secrets.ngrok.authtoken
}

export function getNgrokPublicUrl(): string | undefined {
  return secrets.ngrok.domain
    ? `https://${secrets.ngrok.domain}`
    : undefined
}

// --- One-time payment predicate helpers ---

export function hasStripeOneTimePayment(): boolean {
  return !!(
    secrets.stripe.publishableKey &&
    secrets.stripe.secretKey &&
    secrets.stripe.webhookSecret &&
    secrets.stripe.onetimeProductId
  )
}

export function hasCreemOneTimePayment(): boolean {
  return !!(
    secrets.creem.apiKey &&
    secrets.creem.webhookSecret &&
    secrets.creem.onetimeProductId
  )
}

// --- One-time payment require helpers ---

export function requireStripeOneTimePayment(): void {
  if (!hasStripeOneTimePayment()) {
    throw new Error(
      'Stripe one-time payment credentials not configured. ' +
        'Set STRIPE_PUBLISHABLE_KEY, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, and STRIPE_ONETIME_PRODUCT_ID in demo/.env.demo.',
    )
  }
}

// --- QQ SMTP helpers ---

export function hasQqSmtp(): boolean {
  return !!(
    secrets.qq.smtpHost &&
    secrets.qq.smtpPort &&
    secrets.qq.smtpUsername &&
    secrets.qq.smtpPassword
  )
}

export function requireQqSmtp(): void {
  if (!hasQqSmtp()) {
    throw new Error(
      'QQ SMTP credentials not configured. ' +
        'Set QQ_SMTP_HOST, QQ_SMTP_PORT, QQ_SMTP_USERNAME, QQ_SMTP_PASSWORD in demo/.env.demo. ' +
        'Typical values: QQ_SMTP_HOST=smtp.qq.com, QQ_SMTP_PORT=465, QQ_SMTP_ENCRYPTION=ssl. ' +
        'QQ_SMTP_PASSWORD must be the 16-char authorization code from QQ Mail settings, not the login password.',
    )
  }
}

export function requireCreemOneTimePayment(): void {
  if (!hasCreemOneTimePayment()) {
    throw new Error(
      'Creem one-time payment credentials not configured. ' +
        'Set CREEM_API_KEY, CREEM_WEBHOOK_SECRET, and CREEM_ONETIME_PRODUCT_ID in demo/.env.demo.',
    )
  }
}
