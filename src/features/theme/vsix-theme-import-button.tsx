import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-dialog'
import { Import } from 'lucide-react'
import { toast } from 'sonner'
import type { ThemeSummary, VsixThemeExtractionResult } from '@shared/api/bindings'
import { useExtractVsixThemes } from '@entities/vsix/vsix.query'
import { VsixThemeImportDialog } from '@features/theme/vsix-theme-import-dialog'
import { Button } from '@shared/ui/button'

const VSIX_DIALOG_FILTER = [{ name: 'VSIX', extensions: ['vsix'] }]

type VsixThemeImportButtonProps = {
    themes: ThemeSummary[]
}

export const VsixThemeImportButton: FC<VsixThemeImportButtonProps> = ({ themes }) => {
    const { t } = useTranslation()

    const [extraction, setExtraction] = useState<VsixThemeExtractionResult | null>(null)
    const { mutateAsync: extractVsixThemes, isPending } = useExtractVsixThemes()

    const handleImportClick = async () => {
        const selected = await open({ multiple: false, filters: VSIX_DIALOG_FILTER, title: t('settings.themeImportDialogTitle') })
        if (typeof selected !== 'string') return

        try {
            const result = await extractVsixThemes(selected)
            if (result.themes.length === 0) {
                toast.error(t('settings.themeImportFailure'))
                return
            }
            setExtraction(result)
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('settings.themeImportFailure'))
        }
    }

    return (
        <>
            <Button type='button' variant='outline' size='xs' disabled={isPending} onClick={() => void handleImportClick()}>
                <Import className='size-3.5' />
                {t('settings.themeImportButton')}
            </Button>
            {extraction && (
                <VsixThemeImportDialog
                    open
                    onOpenChange={(next) => !next && setExtraction(null)}
                    result={extraction}
                    existingThemeIds={themes.map((theme) => theme.id)}
                />
            )}
        </>
    )
}
