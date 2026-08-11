import { FolderOpen, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { PluginErrorCode } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { pluginListQueryOptions, useReloadPlugins } from '@entities/plugin/plugin.query'
import { systemOpenAppDataPath } from '@entities/system/system.ipc'

const PLUGIN_ERROR_MESSAGE_KEY: Record<PluginErrorCode, string> = {
    'parse-failed': 'settings.pluginError.parseFailed',
    'id-mismatch': 'settings.pluginError.idMismatch',
    'version-mismatch': 'settings.pluginError.versionMismatch',
    'path-escape': 'settings.pluginError.pathEscape',
}

export const PluginList = () => {
    const { t } = useTranslation()

    const { data: plugins = [], isPending } = useQuery(pluginListQueryOptions())
    const { mutate: reloadPlugins, isPending: isReloading } = useReloadPlugins()

    const handleOpenFolder = () => void systemOpenAppDataPath('plugins').catch((error: Error) => toast.error(error.message))
    const handleReload = () => reloadPlugins(undefined, { onError: (error: Error) => toast.error(error.message) })

    const actions = (
        <div className='flex items-center gap-2'>
            <Button type='button' variant='outline' size='xs' disabled={isReloading} onClick={handleReload}>
                <RefreshCw className={cn('size-3.5', isReloading && 'animate-spin')} />
                {t('settings.pluginsReload')}
            </Button>
            <Button type='button' variant='outline' size='xs' onClick={handleOpenFolder}>
                <FolderOpen className='size-3.5' />
                {t('settings.pluginsOpenFolder')}
            </Button>
        </div>
    )

    if (isPending) return <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>

    if (plugins.length === 0)
        return (
            <div className='flex flex-col gap-3'>
                <div className='border-app-border text-app-sidebar-icon-default flex flex-col gap-1 rounded-md border border-dashed px-3 py-4 text-xs'>
                    <p>{t('settings.pluginsEmpty')}</p>
                    <p>{t('settings.pluginsManifestHint')}</p>
                </div>
                {actions}
            </div>
        )

    return (
        <div className='flex flex-col gap-3'>
            {actions}
            <ul className='flex flex-col gap-1.5'>
                {plugins.map((plugin) => (
                    <li key={plugin.manifest.id} className='border-app-border flex min-w-0 flex-col gap-0.5 rounded-md border px-3 py-2 text-xs'>
                        <div className='flex min-w-0 items-center gap-2'>
                            <span className='text-app-foreground min-w-0 truncate font-medium'>{plugin.manifest.name}</span>
                            <span className='text-app-sidebar-icon-default shrink-0 font-mono'>{plugin.manifest.version}</span>
                            <span
                                className={cn(
                                    'ml-auto shrink-0',
                                    plugin.enabled ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-icon-default',
                                )}>
                                {plugin.enabled ? t('settings.pluginEnabled') : t('settings.pluginDisabled')}
                            </span>
                        </div>
                        {plugin.error && <span className='text-status-error min-w-0'>{t(PLUGIN_ERROR_MESSAGE_KEY[plugin.error])}</span>}
                    </li>
                ))}
            </ul>
        </div>
    )
}
