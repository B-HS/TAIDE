import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { cliInstallStatusQueryOptions, useInstallCliCommand, useUninstallCliCommand } from '@entities/agent/agent.query'
import { IS_MAC } from '@shared/constants/platform'
import { Button } from '@shared/ui/button'

export const AgentCliStatusRow: FC = () => {
    const { t } = useTranslation()

    const { data: status, isPending } = useQuery(cliInstallStatusQueryOptions())
    const { mutate: installCli, isPending: isInstalling } = useInstallCliCommand()
    const { mutate: uninstallCli, isPending: isUninstalling } = useUninstallCliCommand()

    if (!IS_MAC) return null

    const handleInstall = () =>
        installCli(undefined, {
            onSuccess: () => toast.success(t('settings.cliInstallSuccess')),
            onError: () => toast.error(t('settings.cliInstallFailed')),
        })
    const handleUninstall = () =>
        uninstallCli(undefined, {
            onSuccess: () => toast.success(t('settings.cliUninstallSuccess')),
            onError: () => toast.error(t('settings.cliUninstallFailed')),
        })

    return (
        <div className='flex items-center justify-between gap-3 text-xs'>
            <span className='flex min-w-0 flex-col gap-0.5'>
                <span className='text-app-foreground'>{t('keymap.category.shellCommand')}</span>
                {!isPending && status?.installed && (
                    <span className={status.dangling ? 'text-status-warning' : 'text-app-sidebar-icon-default'}>
                        {t(status.dangling ? 'settings.cliStatusDangling' : 'settings.cliStatusInstalled')}
                    </span>
                )}
            </span>
            <div className='flex shrink-0 items-center gap-1.5'>
                {(!status?.installed || status.dangling) && (
                    <Button type='button' variant='outline' size='xs' disabled={isInstalling} onClick={handleInstall}>
                        {t('settings.cliInstallButton')}
                    </Button>
                )}
                {status?.installed && (
                    <Button type='button' variant='outline' size='xs' disabled={isUninstalling} onClick={handleUninstall}>
                        {t('settings.cliUninstallButton')}
                    </Button>
                )}
            </div>
        </div>
    )
}
