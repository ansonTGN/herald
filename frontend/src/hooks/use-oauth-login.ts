import { toast } from 'sonner'
import { oauthLogin } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'

export function useOAuthLogin() {
  async function initiateOAuthLogin(realmId: string, provider: string): Promise<void> {
    try {
      const response = await oauthLogin({ path: { realmId, provider } })

      if (response.error) {
        throw response.error
      }

      const authUrl = response.data?.authUrl
      if (!authUrl) {
        throw new Error('No authUrl in response')
      }

      window.location.href = authUrl
    } catch (error) {
      const errorMessage = getErrorMessage(error)
      console.error('OAuth login failed:', error)
      toast.error(`Failed to initiate OAuth login: ${errorMessage}`)
    }
  }

  return { initiateOAuthLogin }
}
