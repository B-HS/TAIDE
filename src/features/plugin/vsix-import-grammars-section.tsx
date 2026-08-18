import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { CircleCheck } from 'lucide-react'
import { Button } from '@shared/ui/button'

type VsixImportGrammarsSectionProps = {
    imported: boolean
    errorMessage: string | null
    importing: boolean
    onImport: () => void
}

/**
 * Tri-state body of `VsixImportDialog`'s Grammars section — imported / failed / not-yet-attempted.
 * `errorMessage` surfaces `vsix_import_plugin`'s actual Rust error text (e.g. "이미 설치된
 * 플러그인입니다: {id}") instead of a single generic "no language contributions" message, since that
 * failure mode is only one of several (already-installed, missing publisher/name, invalid plugin
 * id, disk I/O) and collapsing them all into it actively misleads a user re-importing a newer
 * version of an extension they've already imported once.
 */
export const VsixImportGrammarsSection: FC<VsixImportGrammarsSectionProps> = ({ imported, errorMessage, importing, onImport }) => {
    const { t } = useTranslation()

    if (imported)
        return (
            <p className='text-status-success flex items-center gap-1.5 text-xs'>
                <CircleCheck className='size-3.5' />
                {t('settings.pluginImportVsixSuccess')}
            </p>
        )

    if (errorMessage !== null) return <p className='text-status-error text-xs'>{errorMessage}</p>

    return (
        <div className='flex justify-end'>
            <Button type='button' variant='outline' size='xs' disabled={importing} onClick={onImport}>
                {t('settings.pluginImportVsixButton')}
            </Button>
        </div>
    )
}
