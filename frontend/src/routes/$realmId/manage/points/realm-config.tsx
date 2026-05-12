import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Info, Loader2 } from 'lucide-react'
import {
  queryKeys,
  realmConfigQueryOptions,
  updateRealmDefaultConfigMutation,
} from '@/data/query-options'
import { realmConfigSchema, type RealmConfigFormData } from '@/lib/schemas/points-forms'

export const Route = createFileRoute('/$realmId/manage/points/realm-config')({
  component: RealmConfigPage,
})

function RealmConfigPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()

  const periodTypeLabels = {
    once: '一次性发放',
    daily: '每日发放',
    weekly: '每周发放',
    monthly: '每月发放',
  }

  const { data: config, isLoading, error } = useQuery(realmConfigQueryOptions(realmId))

  // Treat 404 (no config yet) as non-error: use defaults
  const isNotFound = error && /404|not found|不存在/i.test(error.message || '')
  const effectiveConfig = config ?? null

  const updateMutation = useMutation({
    mutationFn: (data: RealmConfigFormData) =>
      updateRealmDefaultConfigMutation(realmId, {
        registrationBonusPoints: data.registrationBonusPoints,
        freePeriodicPointsAmount: data.freePeriodicPointsAmount,
        freePeriodicGrantPeriodType: data.freePeriodicGrantPeriodType,
        freePeriodicValidityDays: data.freePeriodicValidityDays,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realmConfig(realmId) })
      toast.success('配置已更新')
    },
    onError: (error) => {
      console.error('Failed to update realm config:', error)
      toast.error(error.message || '更新配置失败，请重试')
    },
  })

  const form = useAppForm({
    schema: realmConfigSchema,
    defaultValues: effectiveConfig
      ? {
          ...effectiveConfig,
          freePeriodicGrantPeriodType: effectiveConfig.freePeriodicGrantPeriodType as
            | 'once'
            | 'daily'
            | 'weekly'
            | 'monthly',
        }
      : {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'daily',
          freePeriodicValidityDays: 1,
        },
    onSubmit: async ({ value }) => {
      await updateMutation.mutateAsync(value)
    },
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error && !isNotFound) {
    return (
      <Alert variant="destructive">
        <AlertDescription>加载配置失败: {error.message}</AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">积分默认配置</h1>
        <p className="text-muted-foreground mt-2">
          配置新用户的默认积分设置，包括注册奖励和定期积分
        </p>
      </div>

      <Alert>
        <Info className="h-4 w-4" />
        <AlertDescription>此配置仅影响新注册的用户。现有用户的配置不会受到影响。</AlertDescription>
      </Alert>

      <Card data-testid="realm-config-form" aria-labelledby="realm-config-title">
        <CardHeader>
          <CardTitle id="realm-config-title">默认配置</CardTitle>
          <CardDescription>设置免费用户的积分奖励规则</CardDescription>
        </CardHeader>
        <CardContent>
          <AppForm>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                form.handleSubmit()
              }}
              className="space-y-6"
              aria-label="积分默认配置表单"
            >
              <form.Field name="registrationBonusPoints">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      注册奖励积分 *
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="1000"
                      data-testid="registration-bonus-points-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      用户注册时一次性获得的奖励积分
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="registration-bonus-points-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicGrantPeriodType">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      发放周期类型 *
                    </Label>
                    <Select
                      value={field.state.value}
                      onValueChange={(value) =>
                        field.handleChange(value as 'once' | 'daily' | 'weekly' | 'monthly')
                      }
                      disabled={updateMutation.isPending}
                    >
                      <SelectTrigger
                        id={field.name}
                        data-testid="grant-period-type-select"
                        aria-labelledby={`${field.name}-label`}
                        aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                        aria-invalid={field.state.meta.errors.length > 0}
                        aria-required="true"
                      >
                        <SelectValue placeholder="选择周期类型" />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.entries(periodTypeLabels).map(([value, label]) => (
                          <SelectItem key={value} value={value}>
                            {label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      免费用户积分的发放周期
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="grant-period-type-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicPointsAmount">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      定期积分数量 *
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="50"
                      data-testid="free-periodic-points-amount-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      按选定周期自动发放的积分数量
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="free-periodic-points-amount-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicValidityDays">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      定期积分有效期（天） *
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="1"
                      data-testid="free-periodic-validity-days-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      定期积分的有效期，过期后将自动清除（一次性周期可设置为 0 表示永久有效）
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="free-periodic-validity-days-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              {/* Action Buttons */}
              <div className="flex justify-end pt-4">
                <form.Subscribe
                  selector={(state) => ({
                    canSubmit: state.canSubmit,
                    isSubmitting: state.isSubmitting,
                  })}
                >
                  {(state) => (
                    <Button
                      type="submit"
                      disabled={!state.canSubmit || state.isSubmitting || updateMutation.isPending}
                      data-testid="save-config-button"
                      aria-label={
                        state.isSubmitting || updateMutation.isPending
                          ? '正在保存配置...'
                          : '保存配置'
                      }
                      aria-busy={state.isSubmitting || updateMutation.isPending}
                    >
                      {state.isSubmitting || updateMutation.isPending ? '保存中...' : '保存配置'}
                    </Button>
                  )}
                </form.Subscribe>
              </div>
            </form>
          </AppForm>
        </CardContent>
      </Card>
    </div>
  )
}
