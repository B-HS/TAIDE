import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { TriangleAlert } from 'lucide-react'
import { toast } from 'sonner'
import type { VsixThemeExtractionResult } from '@shared/api/bindings'
import { useSaveTheme } from '@entities/theme/theme.query'
import { generateUniqueThemeId } from '@shared/lib/theme-draft'
import { buildVsixThemeCandidates, type VsixThemeImportFailureReason } from '@features/theme/vsix-theme-import'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Checkbox } from '@shared/ui/checkbox'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'

const THEME_IMPORT_ITEM_FAILURE_KEY: Record<VsixThemeImportFailureReason, string> = {
    parse: 'settings.themeImportThemeParseFailure',
    incomplete: 'settings.themeImportThemeIncomplete',
    contrast: 'settings.themeImportThemeContrastFailure',
}

type VsixThemeImportDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    result: VsixThemeExtractionResult
    existingThemeIds: readonly string[]
}

export const VsixThemeImportDialog: FC<VsixThemeImportDialogProps> = ({ open, onOpenChange, result, existingThemeIds }) => {
    const { t } = useTranslation()
    const { mutateAsync: saveTheme } = useSaveTheme()

    const [candidates] = useState(() => buildVsixThemeCandidates(result, existingThemeIds))
    const [selectedKeys, setSelectedKeys] = useState(
        () => new Set(candidates.filter((candidate) => !candidate.failureReason).map((candidate) => candidate.key)),
    )
    const [isConfirmOverwriteOpen, setIsConfirmOverwriteOpen] = useState(false)
    const [isSaving, setIsSaving] = useState(false)

    const selectedCandidates = candidates.filter((candidate) => selectedKeys.has(candidate.key) && candidate.theme)
    const collidingCandidates = selectedCandidates.filter((candidate) => candidate.idCollides)

    const toggleSelected = (key: string) =>
        setSelectedKeys((prev) => {
            const next = new Set(prev)
            if (next.has(key)) next.delete(key)
            else next.add(key)
            return next
        })

    const persistCandidates = async () => {
        setIsSaving(true)
        try {
            const usedIds = new Set(existingThemeIds)
            for (const candidate of selectedCandidates) {
                if (!candidate.theme) continue
                const shouldCopy = candidate.idCollides
                const id = shouldCopy ? generateUniqueThemeId(candidate.theme.name, [...usedIds]) : candidate.theme.id
                const name = shouldCopy ? t('themeEditor.duplicateNameTemplate', { name: candidate.theme.name }) : candidate.theme.name
                usedIds.add(id)
                await saveTheme({ ...candidate.theme, id, name })
            }
            toast.success(t('settings.themeImportSuccess', { count: selectedCandidates.length, extension: result.extension.displayName }))
            onOpenChange(false)
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('settings.themeImportSaveFailure'))
        } finally {
            setIsSaving(false)
        }
    }

    const handleSaveClick = () => {
        if (collidingCandidates.length > 0) {
            setIsConfirmOverwriteOpen(true)
            return
        }
        void persistCandidates()
    }

    const handleConfirmOverwrite = () => {
        setIsConfirmOverwriteOpen(false)
        void persistCandidates()
    }

    return (
        <>
            <Dialog open={open} onOpenChange={onOpenChange}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{t('settings.themeImportButton')}</DialogTitle>
                        <DialogDescription>{result.extension.displayName}</DialogDescription>
                    </DialogHeader>

                    <ul className='flex max-h-80 flex-col gap-1.5 overflow-y-auto'>
                        {candidates.map((candidate) => (
                            <li
                                key={candidate.key}
                                className={cn(
                                    'border-app-border flex items-center gap-2 rounded-sm border px-2 py-1.5 text-xs',
                                    candidate.failureReason && 'opacity-50',
                                )}>
                                <Checkbox
                                    checked={selectedKeys.has(candidate.key)}
                                    disabled={candidate.failureReason !== null}
                                    onCheckedChange={() => toggleSelected(candidate.key)}
                                />
                                <div className='flex min-w-0 flex-1 flex-col'>
                                    <span className='text-app-foreground truncate'>{candidate.label}</span>
                                    <span className='text-app-sidebar-icon-default flex items-center gap-1.5'>
                                        {candidate.themeType === 'dark' ? t('settings.themeDark') : t('settings.themeLight')}
                                        {candidate.warningCount > 0 && !candidate.failureReason && (
                                            <span className='text-status-warning inline-flex items-center gap-0.5'>
                                                <TriangleAlert className='size-3' />
                                                {candidate.warningCount}
                                            </span>
                                        )}
                                        {candidate.idCollides && !candidate.failureReason && (
                                            <span className='text-status-warning truncate'>
                                                {t('settings.themeImportDuplicate', { name: candidate.id })}
                                            </span>
                                        )}
                                    </span>
                                </div>
                                {candidate.failureReason && (
                                    <span className='text-status-error shrink-0'>{t(THEME_IMPORT_ITEM_FAILURE_KEY[candidate.failureReason])}</span>
                                )}
                            </li>
                        ))}
                    </ul>

                    <DialogFooter>
                        <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                            {t('common.cancel')}
                        </Button>
                        <Button type='button' disabled={selectedCandidates.length === 0 || isSaving} onClick={handleSaveClick}>
                            {t('common.save')}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>

            <AlertDialog open={isConfirmOverwriteOpen} onOpenChange={setIsConfirmOverwriteOpen}>
                <AlertDialogContent size='sm'>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.confirm')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {collidingCandidates.map((candidate) => t('settings.themeImportDuplicate', { name: candidate.id })).join(' / ')}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogAction variant='outline' onClick={() => setIsConfirmOverwriteOpen(false)}>
                            {t('common.cancel')}
                        </AlertDialogAction>
                        <AlertDialogAction variant='default' onClick={handleConfirmOverwrite}>
                            {t('common.confirm')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}
