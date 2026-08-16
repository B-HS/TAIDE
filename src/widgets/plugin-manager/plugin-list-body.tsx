import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { LoadedPlugin, PluginErrorCode } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'
import { Trash2 } from 'lucide-react'

const PLUGIN_ERROR_MESSAGE_KEY: Record<PluginErrorCode, string> = {
    'parse-failed': 'settings.pluginError.parseFailed',
    'id-mismatch': 'settings.pluginError.idMismatch',
    'version-mismatch': 'settings.pluginError.versionMismatch',
    'path-escape': 'settings.pluginError.pathEscape',
    'grammar-missing': 'settings.pluginError.grammarMissing',
    'grammar-invalid': 'settings.pluginError.grammarInvalid',
    'grammar-conflict': 'settings.pluginError.grammarConflict',
}

export type PendingPluginUninstall = { pluginId: string; pluginName: string }

type PluginListBodyProps = {
    isPending: boolean
    plugins: LoadedPlugin[]
    onRequestUninstall: (pending: PendingPluginUninstall) => void
}

export const PluginListBody: FC<PluginListBodyProps> = ({ isPending, plugins, onRequestUninstall }) => {
    const { t } = useTranslation()

    if (isPending) return <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>

    if (plugins.length === 0)
        return (
            <div className='border-app-border text-app-sidebar-icon-default flex flex-col gap-1 rounded-md border border-dashed px-3 py-4 text-xs'>
                <p>{t('settings.pluginsEmpty')}</p>
                <p>{t('settings.pluginsManifestHint')}</p>
            </div>
        )

    return (
        <ul className='flex flex-col gap-1.5'>
            {plugins.map((plugin) => (
                <li key={plugin.manifest.id} className='border-app-border flex min-w-0 flex-col gap-0.5 rounded-md border px-3 py-2 text-xs'>
                    <div className='flex min-w-0 items-center gap-2'>
                        <span className='text-app-foreground min-w-0 truncate font-medium'>{plugin.manifest.name}</span>
                        <span className='text-app-sidebar-icon-default shrink-0 font-mono'>{plugin.manifest.version}</span>
                        <span className={cn('shrink-0', plugin.enabled ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-icon-default')}>
                            {plugin.enabled ? t('settings.pluginEnabled') : t('settings.pluginDisabled')}
                        </span>
                        <IconButton
                            label={t('settings.pluginUninstallButton')}
                            icon={<Trash2 className='size-3.5' />}
                            containerClassName='ml-auto'
                            onClick={() => onRequestUninstall({ pluginId: plugin.manifest.id, pluginName: plugin.manifest.name })}
                        />
                    </div>
                    {plugin.error && <span className='text-status-error min-w-0'>{t(PLUGIN_ERROR_MESSAGE_KEY[plugin.error])}</span>}
                </li>
            ))}
        </ul>
    )
}
