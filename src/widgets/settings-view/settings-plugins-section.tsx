import type { FC } from 'react'
import { lazy, Suspense } from 'react'
import { useTranslation } from 'react-i18next'
import { SettingsSection } from '@features/settings/settings-section'

type SettingsPluginsSectionProps = {
    id: string
}

/**
 * Split from the settings screen's own chunk (audit §1-1): the plugin manager carries the vsix
 * install/import path and the plugin roster UI, none of which the rest of the settings screen
 * needs in order to paint. The section frame renders immediately and only its body streams in.
 */
const PluginManager = lazy(async () => ({ default: (await import('@widgets/plugin-manager/plugin-manager')).PluginManager }))

export const SettingsPluginsSection: FC<SettingsPluginsSectionProps> = ({ id }) => {
    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.plugins')}>
            <Suspense fallback={null}>
                <PluginManager />
            </Suspense>
        </SettingsSection>
    )
}
