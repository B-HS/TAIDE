import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { FolderOpen, Import, RefreshCw } from 'lucide-react'
import { toast } from 'sonner'
import type { VsixThemeExtractionResult } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { pluginListQueryOptions, useReloadPlugins } from '@entities/plugin/plugin.query'
import { useExtractVsixThemes } from '@entities/vsix/vsix.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { systemOpenAppDataPath } from '@entities/system/system.ipc'
import type { PendingPluginUninstall } from '@widgets/plugin-manager/plugin-list-body'
import { PluginListBody } from '@widgets/plugin-manager/plugin-list-body'
import { PluginInstallButton } from '@widgets/plugin-manager/plugin-install-button'
import { PluginUninstallDialog } from '@widgets/plugin-manager/plugin-uninstall-dialog'
import { VsixImportDialog } from '@widgets/plugin-manager/vsix-import-dialog'

const VSIX_IMPORT_DIALOG_FILTER = [{ name: 'VSIX', extensions: ['vsix'] }]

type VsixImportState = { vsixPath: string; result: VsixThemeExtractionResult }

/** Settings screen PLUGINS section (contract §3.4) — list + install/reload/open-folder actions, per-plugin uninstall, and the combined VSIX import dialog (themes + language/grammar). */
export const PluginManager = () => {
    const [pendingUninstall, setPendingUninstall] = useState<PendingPluginUninstall | null>(null)
    const [vsixImportState, setVsixImportState] = useState<VsixImportState | null>(null)

    const { t } = useTranslation()
    const { data: plugins = [], isPending } = useQuery(pluginListQueryOptions())
    const { data: themes = [] } = useQuery(themeListQueryOptions())
    const { mutate: reloadPlugins, isPending: isReloading } = useReloadPlugins()
    const { mutateAsync: extractVsixThemes, isPending: isImportingVsix } = useExtractVsixThemes()

    const handleOpenFolder = () => void systemOpenAppDataPath('plugins').catch((error: Error) => toast.error(error.message))
    const handleReload = () => reloadPlugins(undefined, { onError: (error: Error) => toast.error(error.message) })

    const handleImportVsixClick = async () => {
        const selected = await open({ multiple: false, filters: VSIX_IMPORT_DIALOG_FILTER, title: t('settings.pluginImportVsixDialogTitle') })
        if (typeof selected !== 'string') return
        try {
            const result = await extractVsixThemes(selected)
            setVsixImportState({ vsixPath: selected, result })
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('settings.pluginImportVsixFailed'))
        }
    }

    return (
        <div className='flex flex-col gap-3'>
            <PluginListBody isPending={isPending} plugins={plugins} onRequestUninstall={setPendingUninstall} />

            <div className='flex flex-wrap items-center gap-2'>
                <PluginInstallButton />
                <Button type='button' variant='outline' size='xs' disabled={isImportingVsix} onClick={() => void handleImportVsixClick()}>
                    <Import className='size-3.5' />
                    {t('settings.pluginImportVsixButton')}
                </Button>
                <Button type='button' variant='outline' size='xs' disabled={isReloading} onClick={handleReload}>
                    <RefreshCw className={cn('size-3.5', isReloading && 'animate-spin')} />
                    {t('settings.pluginsReload')}
                </Button>
                <Button type='button' variant='outline' size='xs' onClick={handleOpenFolder}>
                    <FolderOpen className='size-3.5' />
                    {t('settings.pluginsOpenFolder')}
                </Button>
            </div>

            <PluginUninstallDialog pending={pendingUninstall} onOpenChange={(isOpen) => !isOpen && setPendingUninstall(null)} />
            {vsixImportState && (
                <VsixImportDialog
                    open
                    onOpenChange={(isOpen) => !isOpen && setVsixImportState(null)}
                    vsixPath={vsixImportState.vsixPath}
                    result={vsixImportState.result}
                    existingThemeIds={themes.map((theme) => theme.id)}
                />
            )}
        </div>
    )
}
