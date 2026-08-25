import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { FolderOpen } from 'lucide-react'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { useSetThemeId } from '@entities/settings/settings.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'
import { CustomThemeList } from '@features/theme/custom-theme-list'
import { SettingsSection } from '@features/settings/settings-section'
import { SwitchField } from '@features/settings/switch-field'
import { ThemePicker } from '@features/settings/theme-picker'
import type { AppDataPathKind, Settings } from '@shared/api/bindings'
import type { ThemeEditorState } from '@widgets/settings-view/settings-view'
import { Button } from '@shared/ui/button'

type SettingsAppearanceSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
    onOpenAppDataFolder: (kind: AppDataPathKind) => void
    onOpenThemeEditor: (state: ThemeEditorState) => void
}

export const SettingsAppearanceSection: FC<SettingsAppearanceSectionProps> = ({
    id,
    settings,
    updateSettings,
    onOpenAppDataFolder,
    onOpenThemeEditor,
}) => {
    const { data: themes = [], isPending: isThemesPending } = useQuery(themeListQueryOptions())
    const { mutate: setThemeId } = useSetThemeId()

    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.appearance')}>
            {isThemesPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <ThemePicker
                    themes={themes}
                    activeThemeId={settings.themeId ?? ''}
                    onSelect={setThemeId}
                    onDuplicate={(sourceThemeId) => onOpenThemeEditor({ mode: 'create', sourceThemeId })}
                />
            )}
            <SwitchField
                label={t('settings.followSystemTheme')}
                checked={settings.followSystemTheme ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), followSystemTheme: checked })}
            />

            <div className='flex flex-col gap-2 pt-2'>
                <div className='flex items-center justify-between gap-3'>
                    <span className='text-app-sidebar-icon-default text-xs'>{t('themeEditor.customThemes')}</span>
                    <div className='flex items-center gap-2'>
                        <Button variant='outline' size='xs' onClick={() => onOpenAppDataFolder('themes')}>
                            <FolderOpen className='size-3.5' />
                            {t('settings.themesOpenFolder')}
                        </Button>
                        <Button
                            variant='outline'
                            size='xs'
                            onClick={() =>
                                onOpenThemeEditor({
                                    mode: 'create',
                                    sourceThemeId: settings.themeId ?? BUILTIN_THEME_ID.DARK,
                                })
                            }>
                            {t('themeEditor.createNew')}
                        </Button>
                    </div>
                </div>
                {isThemesPending ? (
                    <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                ) : (
                    <CustomThemeList
                        themes={themes.filter((theme) => !theme.builtin)}
                        onEdit={(sourceThemeId) => onOpenThemeEditor({ mode: 'edit', sourceThemeId })}
                        onDuplicate={(sourceThemeId) => onOpenThemeEditor({ mode: 'create', sourceThemeId })}
                    />
                )}
            </div>
        </SettingsSection>
    )
}
