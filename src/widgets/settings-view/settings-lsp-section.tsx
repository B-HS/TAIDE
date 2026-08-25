import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { lspServersQueryOptions, useCancelLspInstall, useInstallLspServer, useLspInstallProgress } from '@entities/lsp/lsp.query'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { SettingsSection } from '@features/settings/settings-section'
import type { LspServerId } from '@shared/api/bindings'
import { ERROR_KEY } from '@shared/constants/error-key'
import { describeIpcError, isIpcErrorKey } from '@shared/lib/ipc-error-message'

type SettingsLspSectionProps = {
    id: string
}

export const SettingsLspSection: FC<SettingsLspSectionProps> = ({ id }) => {
    const { data: lspServers = [], isPending: isLspPending } = useQuery(lspServersQueryOptions())
    const lspInstallProgressByServerId = useLspInstallProgress()
    const { mutate: installLspServer } = useInstallLspServer()
    const { mutate: cancelLspInstall } = useCancelLspInstall()

    const { t } = useTranslation()

    const handleInstallLspServer = (serverId: LspServerId) =>
        installLspServer(serverId, {
            onError: (error) => {
                if (isIpcErrorKey(error, ERROR_KEY.LSP_INSTALL_CANCELLED)) return
                toast.error(describeIpcError(error))
            },
        })
    const handleCancelLspInstall = (serverId: LspServerId) => cancelLspInstall(serverId, { onError: (error) => toast.error(describeIpcError(error)) })

    return (
        <SettingsSection id={id} title={t('settings.lspStatus')} description={t('settings.lspDescription')}>
            {isLspPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <LspServerStatusList
                    servers={lspServers}
                    installProgressByServerId={lspInstallProgressByServerId}
                    onInstall={handleInstallLspServer}
                    onCancelInstall={handleCancelLspInstall}
                />
            )}
        </SettingsSection>
    )
}
