import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Pencil } from 'lucide-react'
import { toast } from 'sonner'
import { aiModelsQueryOptions, aiTokenStatusQueryOptions, useClearAiToken, useSetAiToken } from '@entities/ai/ai.query'
import { useOpenAppFileTab } from '@entities/layout/layout.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { AiAutoTabToggle } from '@features/settings/ai-auto-tab-toggle'
import { AiOmlxRow } from '@features/settings/ai-omlx-row'
import { AiProviderTokenRow } from '@features/settings/ai-provider-token-row'
import { OptionPicker } from '@features/settings/option-picker'
import { SettingsSection } from '@features/settings/settings-section'
import type { AiProviderId, ProjectId, PromptTemplateId, Settings } from '@shared/api/bindings'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { IconButton } from '@shared/ui/icon-button'

/** Default for `Settings.aiProvider` — shared by auto-tab, Inline Edit, and AI commit messages (not auto-tab-only, despite the field's Wave G predecessor name). */
const DEFAULT_AI_PROVIDER: AiProviderId = 'ollamaCloud'

const AI_PROVIDER_OPTIONS = [
    { id: 'ollamaCloud', labelKey: 'settings.aiProviderOllamaCloud' },
    { id: 'codex', labelKey: 'settings.aiProviderCodex' },
    { id: 'omlx', labelKey: 'settings.aiProviderOmlx' },
] as const

const PROMPT_ROWS = [
    { id: 'auto-tab-default', labelKey: 'prompts.autoTabTitle' },
    { id: 'inline-edit-default', labelKey: 'prompts.inlineEditTitle' },
    { id: 'commit-message-default', labelKey: 'prompts.commitMessageTitle' },
] as const

type SettingsAiSectionProps = {
    id: string
    projectId: ProjectId
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
}

export const SettingsAiSection: FC<SettingsAiSectionProps> = ({ id, projectId, settings, updateSettings }) => {
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { mutate: setAiToken, isPending: isSettingAiToken, variables: settingAiTokenVariables } = useSetAiToken()
    const { mutate: clearAiToken } = useClearAiToken()
    const openAppFileTab = useOpenAppFileTab(projectId)

    const selectedAiProvider = settings.aiProvider ?? DEFAULT_AI_PROVIDER
    const isSelectedAiProviderConfigured = aiTokenStatus?.[selectedAiProvider] ?? false
    const {
        data: aiModels = [],
        isPending: isAiModelsPending,
        isError: isAiModelsError,
    } = useQuery(aiModelsQueryOptions(isSelectedAiProviderConfigured ? selectedAiProvider : null))

    const { t } = useTranslation()

    const handleSaveAiToken = (provider: AiProviderId, token: string) =>
        setAiToken({ provider, token }, { onError: (error) => toast.error(describeIpcError(error) || t('settings.aiTokenSaveFailed')) })
    const handleClearAiToken = (provider: AiProviderId) =>
        clearAiToken(provider, { onError: (error) => toast.error(describeIpcError(error) || t('settings.aiTokenClearFailed')) })
    const handleOmlxBaseUrlCommit = (value: string) => updateSettings({ ...emptySettingsPatch(), aiOmlxBaseUrl: value.trim() })

    const handleOpenPromptFile = (promptId: PromptTemplateId, labelKey: string) => openAppFileTab({ kind: 'prompt', id: promptId }, t(labelKey))

    return (
        <SettingsSection id={id} title={t('settings.aiSectionTitle')}>
            <ul className='flex flex-col gap-1.5'>
                <AiProviderTokenRow
                    label={t('settings.aiProviderOllamaCloud')}
                    configured={aiTokenStatus?.ollamaCloud ?? false}
                    saving={isSettingAiToken && settingAiTokenVariables?.provider === 'ollamaCloud'}
                    onSave={(token) => handleSaveAiToken('ollamaCloud', token)}
                    onClear={() => handleClearAiToken('ollamaCloud')}
                />
                <AiProviderTokenRow
                    label={t('settings.aiProviderCodex')}
                    warning={t('settings.aiCodexUnofficialWarning')}
                    configured={aiTokenStatus?.codex ?? false}
                    saving={isSettingAiToken && settingAiTokenVariables?.provider === 'codex'}
                    onSave={(token) => handleSaveAiToken('codex', token)}
                    onClear={() => handleClearAiToken('codex')}
                />
                <AiOmlxRow
                    baseUrl={settings.aiOmlxBaseUrl ?? ''}
                    onBaseUrlCommit={handleOmlxBaseUrlCommit}
                    apiKeySaving={isSettingAiToken && settingAiTokenVariables?.provider === 'omlx'}
                    onSaveApiKey={(token) => handleSaveAiToken('omlx', token)}
                    onClearApiKey={() => handleClearAiToken('omlx')}
                />
            </ul>
            <OptionPicker
                label={t('settings.aiProviderLabel')}
                options={AI_PROVIDER_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={selectedAiProvider}
                onSelect={(providerId) => updateSettings({ ...emptySettingsPatch(), aiProvider: providerId, aiModel: '' })}
            />
            {isSelectedAiProviderConfigured && isAiModelsError && (
                <span className='text-status-error text-xs'>
                    {t(selectedAiProvider === 'omlx' ? 'settings.aiOmlxConnectFailed' : 'settings.aiModelLoadFailed')}
                </span>
            )}
            {isSelectedAiProviderConfigured && !isAiModelsError && !isAiModelsPending && aiModels.length === 0 && (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.aiModelSelectPlaceholder')}</span>
            )}
            {isSelectedAiProviderConfigured && !isAiModelsError && aiModels.length > 0 && (
                <OptionPicker
                    label={t('settings.aiModelLabel')}
                    options={aiModels.map((model) => ({ id: model.modelId, label: model.displayName ?? model.modelId }))}
                    value={settings.aiModel ?? ''}
                    placeholder={t('settings.aiModelSelectPlaceholder')}
                    onSelect={(modelId) => updateSettings({ ...emptySettingsPatch(), aiProvider: selectedAiProvider, aiModel: modelId })}
                />
            )}
            <AiAutoTabToggle
                checked={settings.aiAutoTabEnabled ?? false}
                disabled={!isSelectedAiProviderConfigured}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), aiProvider: selectedAiProvider, aiAutoTabEnabled: checked })}
            />
            <ul className='border-app-border flex flex-col gap-1.5 border-t pt-3'>
                {PROMPT_ROWS.map((row) => (
                    <li key={row.id} className='flex items-center justify-between gap-3 text-xs'>
                        <span className='text-app-foreground'>{t(row.labelKey)}</span>
                        <IconButton
                            label={t('prompts.editEntry')}
                            icon={<Pencil className='size-3.5' />}
                            onClick={() => handleOpenPromptFile(row.id, row.labelKey)}
                        />
                    </li>
                ))}
            </ul>
        </SettingsSection>
    )
}
