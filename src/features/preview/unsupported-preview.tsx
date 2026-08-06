import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { ExternalLink, FileQuestion } from 'lucide-react'
import { Button } from '@shared/ui/button'

export type UnsupportedPreviewProps = {
    fileName: string
    onOpenExternal: () => void
}

export const UnsupportedPreview: FC<UnsupportedPreviewProps> = ({ fileName, onOpenExternal }) => {
    const { t } = useTranslation()

    return (
        <div className='bg-editor-background text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-3 p-4 text-sm'>
            <FileQuestion className='size-10' />
            <div className='flex flex-col items-center gap-1'>
                <span>{t('preview.notSupported')}</span>
                <span className='text-editor-foreground max-w-md truncate text-xs opacity-70'>{fileName}</span>
            </div>
            <Button type='button' variant='outline' size='sm' onClick={onOpenExternal}>
                <ExternalLink />
                {t('preview.openExternally')}
            </Button>
        </div>
    )
}
