import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'
import { TextField } from '@features/settings/text-field'

type AiOmlxRowProps = {
    baseUrl: string
    onBaseUrlCommit: (value: string) => void
    apiKeySaving: boolean
    onSaveApiKey: (token: string) => void
    onClearApiKey: () => void
}

export const AiOmlxRow: FC<AiOmlxRowProps> = ({ baseUrl, onBaseUrlCommit, apiKeySaving, onSaveApiKey, onClearApiKey }) => {
    const { t } = useTranslation()
    const [apiKey, setApiKey] = useState('')

    const handleSaveApiKey = () => {
        const trimmed = apiKey.trim()
        if (!trimmed) return
        setApiKey('')
        onSaveApiKey(trimmed)
    }

    return (
        <li className='border-app-border flex min-w-0 flex-col gap-1.5 rounded-md border px-3 py-2 text-xs'>
            <TextField
                label={t('settings.aiOmlxBaseUrlLabel')}
                value={baseUrl}
                placeholder={t('settings.aiOmlxBaseUrlPlaceholder')}
                onCommit={onBaseUrlCommit}
            />
            <div className='flex min-w-0 items-center gap-1.5'>
                <input
                    type='password'
                    value={apiKey}
                    placeholder={t('settings.aiOmlxApiKeyOptional')}
                    onChange={(event) => setApiKey(event.target.value)}
                    className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                />
                <Button type='button' variant='outline' size='xs' disabled={!apiKey.trim() || apiKeySaving} onClick={handleSaveApiKey}>
                    {t('settings.aiTokenSave')}
                </Button>
                <Button type='button' variant='ghost' size='xs' onClick={onClearApiKey}>
                    {t('settings.aiTokenClear')}
                </Button>
            </div>
        </li>
    )
}
