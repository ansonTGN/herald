import type { PublicWhiteLabelConfig } from '@/lib/api-generated/types.gen'
import { BRAND_NAME } from '@/lib/constants'

export function resolveBrandName(
  whiteLabel: PublicWhiteLabelConfig | null | undefined,
  realmName?: string | null
): string {
  return whiteLabel?.brandName?.trim() || realmName?.trim() || BRAND_NAME
}
