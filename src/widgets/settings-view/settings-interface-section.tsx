import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { projectListQueryOptions } from '@entities/project/project.query'
import { AgentCliStatusRow } from '@widgets/settings-view/agent-cli-status-row'
import { AgentHooksProjectList } from '@widgets/settings-view/agent-hooks-project-list'
import { AgentHooksToggle } from '@features/settings/agent-hooks-toggle'
import { NumericField } from '@features/settings/numeric-field'
import { SettingsSection } from '@features/settings/settings-section'
import { SwitchField } from '@features/settings/switch-field'
import { ToastPositionPicker } from '@features/settings/toast-position-picker'
import { DEFAULT_RESIZER_THICKNESS, MAX_RESIZER_THICKNESS, MIN_RESIZER_THICKNESS } from '@shared/constants/layout'
import { DEFAULT_TOAST_POSITION } from '@shared/constants/toast'
import type { Settings } from '@shared/api/bindings'

type SettingsInterfaceSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
}

export const SettingsInterfaceSection: FC<SettingsInterfaceSectionProps> = ({ id, settings, updateSettings }) => {
    const { data: projects = [] } = useQuery(projectListQueryOptions())

    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.interface')}>
            <div className='flex flex-col gap-2'>
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.toastPosition')}</span>
                <ToastPositionPicker
                    value={settings.toastPosition ?? DEFAULT_TOAST_POSITION}
                    translateLabel={t}
                    onSelect={(toastPosition) => updateSettings({ ...emptySettingsPatch(), toastPosition })}
                />
            </div>
            <NumericField
                label={t('settings.resizerThickness')}
                value={settings.resizerThickness ?? DEFAULT_RESIZER_THICKNESS}
                min={MIN_RESIZER_THICKNESS}
                max={MAX_RESIZER_THICKNESS}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), resizerThickness: value })}
            />
            <SwitchField
                label={t('settings.showSystemUsage')}
                checked={settings.showSystemUsage ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), showSystemUsage: checked })}
            />
            <SwitchField
                label={t('settings.editorMinimap')}
                checked={settings.editorMinimap ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorMinimap: checked })}
            />
            <SwitchField
                label={t('settings.agentStatusBadge')}
                checked={settings.agentStatusBadgeEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), agentStatusBadgeEnabled: checked })}
            />
            <AgentCliStatusRow />
            <div className='flex flex-col gap-2'>
                <AgentHooksToggle
                    label={t('settings.agentHooks')}
                    hint={t('settings.agentHooksHint')}
                    checked={settings.agentHooksEnabled ?? false}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), agentHooksEnabled: checked })}
                />
                {(settings.agentHooksEnabled ?? false) && <AgentHooksProjectList projects={projects} />}
            </div>
            <SwitchField
                label={t('settings.ideIntegration')}
                description={t('settings.ideIntegrationHint')}
                checked={settings.ideIntegrationEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), ideIntegrationEnabled: checked })}
            />
            <SwitchField
                label={t('settings.ideAutoOpenDiff')}
                checked={settings.ideAutoOpenDiff ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), ideAutoOpenDiff: checked })}
            />
            <SwitchField
                label={t('settings.enablePreviewTabs')}
                description={t('settings.enablePreviewTabsHint')}
                checked={settings.enablePreviewTabs ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), enablePreviewTabs: checked })}
            />
            <SwitchField
                label={t('settings.explorerAutoReveal')}
                description={t('settings.explorerAutoRevealDescription')}
                checked={settings.explorerAutoReveal ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), explorerAutoReveal: checked })}
            />
            <SwitchField
                label={t('settings.zenFullscreen')}
                description={t('settings.zenFullscreenDescription')}
                checked={settings.zenFullscreen ?? false}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), zenFullscreen: checked })}
            />
            <SwitchField
                label={t('settings.zenHideStatusBar')}
                description={t('settings.zenHideStatusBarDescription')}
                checked={settings.zenHideStatusBar ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), zenHideStatusBar: checked })}
            />
        </SettingsSection>
    )
}
