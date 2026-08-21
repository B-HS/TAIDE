import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { SettingsSection } from '@features/settings/settings-section'
import { PluginManager } from '@widgets/plugin-manager/plugin-manager'

type SettingsPluginsSectionProps = {
    id: string
}

export const SettingsPluginsSection: FC<SettingsPluginsSectionProps> = ({ id }) => {
    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.plugins')}>
            <PluginManager />
        </SettingsSection>
    )
}
