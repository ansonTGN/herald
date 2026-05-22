import { useCallback } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Copy, Check } from 'lucide-react'
import { useCopyToClipboard } from '@/hooks/use-copy-to-clipboard'

interface BackupCodesDisplayProps {
  backupCodes: string[]
}

export function BackupCodesDisplay({ backupCodes }: BackupCodesDisplayProps) {
  const { copied: allCopied, copyToClipboard } = useCopyToClipboard()

  const handleCopyAll = useCallback(async () => {
    await copyToClipboard(backupCodes.join('\n'))
  }, [backupCodes, copyToClipboard])

  return (
    <Card data-testid="backup-codes-display">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <div className="flex flex-col space-y-1.5">
          <CardTitle>Backup Recovery Codes</CardTitle>
          <CardDescription className="text-destructive">
            These codes will only be shown once. Save them securely!
          </CardDescription>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={handleCopyAll}
          data-testid="backup-codes-copy-all-button"
          className="shrink-0"
        >
          {allCopied ? (
            <>
              <Check className="h-4 w-4 mr-2" />
              Copied
            </>
          ) : (
            <>
              <Copy className="h-4 w-4 mr-2" />
              Copy All
            </>
          )}
        </Button>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          {backupCodes.map((code, index) => (
            <code key={index} className="font-mono text-sm" data-testid={`backup-code-${index}`}>
              {code}
            </code>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
