import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { SettingsSection } from '@features/settings/settings-section'
import { requestOpenKeybindingsEditor } from '@shared/lib/keymap/keybindings-bridge'
import { Button } from '@shared/ui/button'

type SettingsKeymapSectionProps = {
    id: string
}

export const SettingsKeymapSection: FC<SettingsKeymapSectionProps> = ({ id }) => {
    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.keymap')} description={t('settings.keymapDescription')}>
            <Button type='button' variant='outline' size='sm' onClick={() => requestOpenKeybindingsEditor()}>
                {t('settings.keymapOpenEditor')}
            </Button>
        </SettingsSection>
    )
}
