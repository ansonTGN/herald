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
  },
  stripe: {
    publishableKey: process.env.STRIPE_PUBLISHABLE_KEY,
    secretKey: process.env.STRIPE_SECRET_KEY,
    webhookSecret: process.env.STRIPE_WEBHOOK_SECRET,
  },
  wechat: {
    appId: process.env.WECHAT_APP_ID,
    mchId: process.env.WECHAT_MCH_ID,
    v3Key: process.env.WECHAT_V3_KEY,
    serialNo: process.env.WECHAT_SERIAL_NO,
    privateKey: process.env.WECHAT_PRIVATE_KEY,
    notifyUrl: process.env.WECHAT_NOTIFY_URL,
  },
  ngrok: {
    authtoken: process.env.NGROK_AUTHTOKEN,
    domain: process.env.NGROK_DOMAIN,
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
    secrets.stripe.webhookSecret
  )
}

export function hasWechatPayment(): boolean {
  return !!(
    secrets.wechat.appId &&
    secrets.wechat.mchId &&
    secrets.wechat.v3Key &&
    secrets.wechat.serialNo &&
    secrets.wechat.privateKey
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
        'Set STRIPE_PUBLISHABLE_KEY, STRIPE_SECRET_KEY, and STRIPE_WEBHOOK_SECRET in demo/.env.demo.',
    )
  }
}

export function requireWechatPayment(): void {
  if (!hasWechatPayment()) {
    throw new Error(
      'WeChat Pay credentials not configured. ' +
        'Set WECHAT_APP_ID, WECHAT_MCH_ID, WECHAT_V3_KEY, WECHAT_SERIAL_NO, and WECHAT_PRIVATE_KEY in demo/.env.demo.',
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
