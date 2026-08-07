import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { Button } from '@shared/ui/button'
import { agentHooksStatusQueryOptions, useInstallAgentHooks, useUninstallAgentHooks } from '@entities/agent/agent.query'

type AgentHooksProjectRowProps = {
    projectId: ProjectId
    projectName: string
}

export const AgentHooksProjectRow: FC<AgentHooksProjectRowProps> = ({ projectId, projectName }) => {
    const [isConsentOpen, setIsConsentOpen] = useState(false)

    const { data: status, isPending } = useQuery(agentHooksStatusQueryOptions(projectId))
    const { mutate: installHooks, isPending: isInstalling } = useInstallAgentHooks()
    const { mutate: uninstallHooks, isPending: isUninstalling } = useUninstallAgentHooks()

    const { t } = useTranslation()

    const handleConfirmInstall = () => {
        installHooks(projectId)
        setIsConsentOpen(false)
    }

    return (
        <li className='border-app-border flex min-w-0 items-center justify-between gap-3 rounded-sm border px-3 py-1.5 text-xs'>
            <span className='min-w-0 truncate text-app-foreground'>{projectName}</span>

            {isPending ? (
                <span className='text-app-sidebar-icon-default shrink-0'>{t('settings.loading')}</span>
            ) : status?.installed ? (
                <div className='flex shrink-0 items-center gap-2'>
                    <span className='text-app-sidebar-icon-agent-running'>{t('agent.hooksInstalled')}</span>
                    <Button type='button' variant='outline' size='xs' disabled={isUninstalling} onClick={() => uninstallHooks(projectId)}>
                        {t('agent.hooksUninstall')}
                    </Button>
                </div>
            ) : (
                <Button type='button' variant='outline' size='xs' disabled={isInstalling} className='shrink-0' onClick={() => setIsConsentOpen(true)}>
                    {t('agent.hooksInstall')}
                </Button>
            )}

            <AlertDialog open={isConsentOpen} onOpenChange={setIsConsentOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('agent.hooksConsentTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>{t('agent.hooksConsentDescription')}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={handleConfirmInstall}>{t('agent.hooksInstall')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </li>
    )
}
