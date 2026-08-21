import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { FolderOpen } from 'lucide-react'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { localeListQueryOptions } from '@entities/locale/locale.query'
import { LanguagePicker, SYSTEM_LANGUAGE_ID } from '@features/settings/language-picker'
import { SettingsSection } from '@features/settings/settings-section'
import type { AppDataPathKind, Settings } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'

type SettingsLanguageSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
    onOpenAppDataFolder: (kind: AppDataPathKind) => void
}

export const SettingsLanguageSection: FC<SettingsLanguageSectionProps> = ({ id, settings, updateSettings, onOpenAppDataFolder }) => {
    const { data: locales = [], isPending: isLocalesPending } = useQuery(localeListQueryOptions())

    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.language')}>
            {isLocalesPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <LanguagePicker
                    locales={locales}
                    activeLanguage={settings.language ?? SYSTEM_LANGUAGE_ID}
                    systemLabel={t('settings.systemLanguage')}
                    onSelect={(language) => updateSettings({ ...emptySettingsPatch(), language })}
                />
            )}
            <div className='flex justify-end'>
                <Button variant='outline' size='xs' onClick={() => onOpenAppDataFolder('locales')}>
                    <FolderOpen className='size-3.5' />
                    {t('settings.localesOpenFolder')}
                </Button>
            </div>
        </SettingsSection>
    )
}
