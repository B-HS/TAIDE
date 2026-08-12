import type { FC } from 'react'
import { CheckCircle2, Copy, Loader2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { LspInstallProgress, LspServerDetection, LspServerId } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Progress } from '@shared/ui/progress'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const ALPHA_LSP_SERVER_IDS: readonly string[] = ['expert']

const BYTES_PER_KIB = 1024
const BYTES_PER_MIB = BYTES_PER_KIB * 1024

const formatBytes = (bytes: number | null) => {
    const value = bytes ?? 0
    if (value < BYTES_PER_KIB) return `${value}B`
    if (value < BYTES_PER_MIB) return `${(value / BYTES_PER_KIB).toFixed(1)}KB`
    return `${(value / BYTES_PER_MIB).toFixed(1)}MB`
}

type LspServerStatusListProps = {
    servers: LspServerDetection[]
    installProgressByServerId: Record<string, LspInstallProgress>
    onInstall: (serverId: LspServerId) => void
    onCancelInstall: (serverId: LspServerId) => void
}

type LspServerRowProps = {
    server: LspServerDetection
    progress: LspInstallProgress | undefined
    onInstall: (serverId: LspServerId) => void
    onCancelInstall: (serverId: LspServerId) => void
}

const LspInstallProgressLine: FC<{ progress: LspInstallProgress }> = ({ progress }) => {
    const { t } = useTranslation()

    if (progress.phase === 'failed') return <span className='text-status-error min-w-0'>{progress.message ?? t('settings.lspInstallFailed')}</span>

    return (
        <div className='flex min-w-0 flex-col gap-1'>
            <span className='text-app-sidebar-icon-default flex items-center gap-1.5'>
                <Loader2 className='size-3 shrink-0 animate-spin' />
                {t('settings.lspInstalling')}
                {progress.phase === 'downloading' && progress.totalBytes != null && (
                    <span className='font-mono'>
                        {formatBytes(progress.receivedBytes)} / {formatBytes(progress.totalBytes)}
                    </span>
                )}
            </span>
            {progress.phase === 'downloading' && progress.totalBytes != null && (
                <Progress value={((progress.receivedBytes ?? 0) / progress.totalBytes) * 100} className='h-1' />
            )}
        </div>
    )
}

const LspServerRow: FC<LspServerRowProps> = ({ server, progress, onInstall, onCancelInstall }) => {
    const { t } = useTranslation()

    const handleCopyCommand = async (command: string) => {
        try {
            await navigator.clipboard.writeText(command)
            toast.success(t('settings.lspCommandCopied'))
        } catch {
            toast.error(t('settings.lspCommandCopyFailed'))
        }
    }

    const isFailed = progress?.phase === 'failed'
    const isActive = progress != null && progress.phase !== 'done' && !isFailed
    const isAlpha = ALPHA_LSP_SERVER_IDS.includes(server.id)

    const renderInstallAction = () => {
        if (server.available) return null
        if (isActive)
            return (
                <Button type='button' variant='ghost' size='xs' onClick={() => onCancelInstall(server.id)}>
                    {t('settings.lspCancel')}
                </Button>
            )

        if (server.installStrategy === 'download') {
            const canInstall = server.downloadAvailable === true
            return (
                <Button type='button' variant='outline' size='xs' disabled={!canInstall} onClick={() => onInstall(server.id)}>
                    {t('settings.lspInstall')}
                </Button>
            )
        }

        if (server.installStrategy === 'toolchain' && server.toolchainAvailable === true)
            return (
                <Button type='button' variant='outline' size='xs' onClick={() => onInstall(server.id)}>
                    {server.toolchainTool} {t('settings.lspInstall')}
                </Button>
            )

        return null
    }

    const renderStatusHint = () => {
        if (server.available || isActive) return null

        if (server.installStrategy === 'download' && server.downloadAvailable !== true)
            return <span className='text-status-warning min-w-0'>{t('settings.lspChecksumPending')}</span>

        if (server.installStrategy === 'toolchain' && server.toolchainAvailable !== true)
            return <span className='text-status-warning min-w-0'>{t('settings.lspToolchainMissing')}</span>

        if (server.installStrategy === 'sdk-detect' && server.sdkAvailable !== true)
            return <span className='text-status-warning min-w-0'>{t('settings.lspSdkMissing')}</span>

        return null
    }

    const showHintCopy = !server.available && !isActive && server.installStrategy !== 'sdk-detect' && server.installHint
    const showPlainHint = !server.available && !isActive && server.installStrategy === 'sdk-detect' && server.installHint

    return (
        <li className='border-app-border flex min-w-0 flex-col gap-1 rounded-md border px-3 py-2 text-xs'>
            <div className='flex min-w-0 items-center gap-2'>
                {server.available ? (
                    <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                ) : (
                    <XCircle className='text-app-sidebar-badge size-3.5 shrink-0' />
                )}
                <span className='text-app-foreground min-w-0 truncate font-medium'>{server.name}</span>
                {isAlpha && (
                    <span className='text-status-warning border-status-warning/40 shrink-0 rounded border px-1 text-[10px]'>
                        {t('settings.lspExperimental')}
                    </span>
                )}
                <span className={cn('ml-auto shrink-0', server.available ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-badge')}>
                    {server.available
                        ? server.installedVersion
                            ? `${t('settings.lspInstalled')} (v${server.installedVersion})`
                            : t('settings.lspInstalled')
                        : t('settings.lspNotInstalled')}
                </span>
                {renderInstallAction()}
            </div>
            {server.resolvedPath && <span className='text-app-sidebar-icon-default min-w-0 pl-5.5 font-mono break-all'>{server.resolvedPath}</span>}
            {(isActive || isFailed) && progress && (
                <div className='pl-5.5'>
                    <LspInstallProgressLine progress={progress} />
                </div>
            )}
            {!isActive && !isFailed && renderStatusHint() && <div className='pl-5.5'>{renderStatusHint()}</div>}
            {showPlainHint && <span className='text-app-sidebar-icon-default min-w-0 pl-5.5'>{server.installHint}</span>}
            {showHintCopy && (
                <div className='flex min-w-0 items-center gap-1.5 pl-5.5'>
                    <span className='text-app-sidebar-icon-default min-w-0'>{server.installHint}</span>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                type='button'
                                variant='ghost'
                                size='icon-xs'
                                aria-label={t('settings.lspCopyCommand')}
                                onClick={() => void handleCopyCommand(server.installHint ?? '')}>
                                <Copy className='size-3' />
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side='bottom'>{t('settings.lspCopyCommand')}</TooltipContent>
                    </Tooltip>
                </div>
            )}
        </li>
    )
}

export const LspServerStatusList: FC<LspServerStatusListProps> = ({ servers, installProgressByServerId, onInstall, onCancelInstall }) => (
    <ul className='flex flex-col gap-1.5'>
        {servers.map((server) => (
            <LspServerRow
                key={server.id}
                server={server}
                progress={installProgressByServerId[server.id]}
                onInstall={onInstall}
                onCancelInstall={onCancelInstall}
            />
        ))}
    </ul>
)
