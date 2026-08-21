import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { FolderOpen } from 'lucide-react'
import { SettingsSection } from '@features/settings/settings-section'
import type { AppDataPathKind } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'

type SettingsSnippetsSectionProps = {
    id: string
    onManage: () => void
    onOpenAppDataFolder: (kind: AppDataPathKind) => void
}

export const SettingsSnippetsSection: FC<SettingsSnippetsSectionProps> = ({ id, onManage, onOpenAppDataFolder }) => {
    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.snippetsSectionTitle')}>
            <div className='flex items-center gap-2'>
                <Button type='button' variant='outline' size='sm' onClick={onManage}>
                    {t('settings.snippetsManage')}
                </Button>
                <Button type='button' variant='outline' size='xs' onClick={() => onOpenAppDataFolder('snippets')}>
                    <FolderOpen className='size-3.5' />
                    {t('settings.snippetsOpenFolder')}
                </Button>
            </div>
        </SettingsSection>
    )
}
