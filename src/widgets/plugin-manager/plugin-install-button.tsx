import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-dialog'
import { Import } from 'lucide-react'
import { toast } from 'sonner'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { Button } from '@shared/ui/button'
import { useInstallPlugin } from '@entities/plugin/plugin.query'

const PLUGIN_ARCHIVE_DIALOG_FILTER = [{ name: 'Plugin Archive', extensions: ['zip'] }]

/**
 * Installs a plugin from a `.zip` archive that already contains a top-level `taide-plugin.json` —
 * a TAIDE-native plugin package, not a real VS Code extension. `.vsix` is deliberately not offered
 * here even though `plugin_install`'s archive path can technically open one as a zip: a real
 * `.vsix` has no `taide-plugin.json` (it's `extension/package.json`), so picking one through this
 * dialog always fails with a confusing "taide-plugin.json을(를) 찾을 수 없습니다" error. Importing an
 * actual VS Code extension's grammars goes through the separate "Import from VSIX" flow
 * (`plugin-manager.tsx`'s `handleImportVsixClick` → `vsix_import_plugin`), which synthesizes a
 * `taide-plugin.json` from the extension's `contributes` instead of requiring one to already exist.
 * `plugin_install` also accepts a plain directory (contract §3.4), but Tauri's `open()` dialog can
 * only be configured for file selection or directory selection, never both in one native call — a
 * directory install stays reachable by placing the plugin's folder directly into the app's plugins
 * directory (already exposed via "Open Plugins Folder"), just without a dedicated folder-picker
 * affordance here.
 */
export const PluginInstallButton = () => {
    const { t } = useTranslation()
    const { mutate: installPlugin, isPending } = useInstallPlugin()

    const handleClick = async () => {
        const selected = await open({ multiple: false, filters: PLUGIN_ARCHIVE_DIALOG_FILTER, title: t('settings.pluginInstallDialogTitle') })
        if (typeof selected !== 'string') return

        installPlugin(selected, {
            onSuccess: () => toast.success(t('settings.pluginInstallSuccess')),
            onError: (error) => toast.error(describeIpcError(error) || t('settings.pluginInstallFailed')),
        })
    }

    return (
        <Button type='button' variant='outline' size='xs' disabled={isPending} onClick={() => void handleClick()}>
            <Import className='size-3.5' />
            {t('settings.pluginInstallButton')}
        </Button>
    )
}
