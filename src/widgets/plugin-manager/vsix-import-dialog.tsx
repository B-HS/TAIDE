import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { TriangleAlert } from 'lucide-react'
import { toast } from 'sonner'
import type { VsixThemeExtractionResult } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { generateUniqueThemeId } from '@shared/lib/theme-draft'
import { useSaveTheme } from '@entities/theme/theme.query'
import { useImportVsixPlugin } from '@entities/vsix/vsix.query'
import { buildVsixThemeCandidates, type VsixThemeImportFailureReason } from '@shared/lib/vsix-theme-import'
import { VsixImportGrammarsSection } from '@features/plugin/vsix-import-grammars-section'
import { Button } from '@shared/ui/button'
import { Checkbox } from '@shared/ui/checkbox'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
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

type VsixImportDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    vsixPath: string
    result: VsixThemeExtractionResult
    existingThemeIds: readonly string[]
}

/**
 * Combined VSIX import surface (contract §3.4) — the Themes section is the pre-existing
 * `vsix-theme-import-dialog.tsx` candidate-selection flow (unchanged logic, now embedded), and the
 * Grammars section is a single all-or-nothing action: `vsix_import_plugin` has no partial-selection
 * mode (it lands every `contributes.languages`/`contributes.grammars` entry as one plugin), so there
 * is nothing to check off individually there, only an import/imported/failed tri-state. A failure
 * surfaces `vsix_import_plugin`'s actual error text (see `VsixImportGrammarsSection`) rather than a
 * single generic message, since "no language contributions" is only one of several distinct failure
 * modes (already installed, missing publisher/name, invalid plugin id, disk I/O).
 */
export const VsixImportDialog: FC<VsixImportDialogProps> = ({ open, onOpenChange, vsixPath, result, existingThemeIds }) => {
    const [candidates] = useState(() => buildVsixThemeCandidates(result, existingThemeIds))
    const [selectedKeys, setSelectedKeys] = useState(
        () => new Set(candidates.filter((candidate) => !candidate.failureReason).map((candidate) => candidate.key)),
    )
    const [isConfirmOverwriteOpen, setIsConfirmOverwriteOpen] = useState(false)
    const [isSavingThemes, setIsSavingThemes] = useState(false)

    const { t } = useTranslation()
    const { mutateAsync: saveTheme } = useSaveTheme()
    const { mutate: importVsixPlugin, isPending: isImportingGrammars, isSuccess: grammarsImported, error: grammarImportError } = useImportVsixPlugin()

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
        setIsSavingThemes(true)
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
        } catch (error) {
            toast.error(error instanceof Error ? error.message : t('settings.themeImportSaveFailure'))
        } finally {
            setIsSavingThemes(false)
        }
    }

    const handleSaveThemesClick = () => {
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

    const handleImportGrammarsClick = () => importVsixPlugin(vsixPath, { onSuccess: () => toast.success(t('settings.pluginImportVsixSuccess')) })

    return (
        <>
            <Dialog open={open} onOpenChange={onOpenChange}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{t('settings.pluginImportVsixButton')}</DialogTitle>
                        <DialogDescription>{result.extension.displayName}</DialogDescription>
                    </DialogHeader>

                    <section className='flex flex-col gap-2'>
                        <h3 className='text-app-sidebar-icon-default text-xs font-medium'>{t('settings.pluginImportVsixThemesSection')}</h3>
                        {candidates.length === 0 ? (
                            <p className='text-app-sidebar-icon-default text-xs'>{t('settings.pluginImportVsixNoThemes')}</p>
                        ) : (
                            <>
                                <ul className='flex max-h-64 flex-col gap-1.5 overflow-y-auto'>
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
                                                <span className='text-status-error shrink-0'>
                                                    {t(THEME_IMPORT_ITEM_FAILURE_KEY[candidate.failureReason])}
                                                </span>
                                            )}
                                        </li>
                                    ))}
                                </ul>
                                <div className='flex justify-end'>
                                    <Button
                                        type='button'
                                        size='xs'
                                        disabled={selectedCandidates.length === 0 || isSavingThemes}
                                        onClick={handleSaveThemesClick}>
                                        {t('common.save')}
                                    </Button>
                                </div>
                            </>
                        )}
                    </section>

                    <section className='border-app-border flex flex-col gap-2 border-t pt-3'>
                        <h3 className='text-app-sidebar-icon-default text-xs font-medium'>{t('settings.pluginImportVsixGrammarsSection')}</h3>
                        <VsixImportGrammarsSection
                            imported={grammarsImported}
                            errorMessage={grammarImportError instanceof Error ? grammarImportError.message : null}
                            importing={isImportingGrammars}
                            onImport={handleImportGrammarsClick}
                        />
                    </section>

                    <DialogFooter>
                        <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                            {t('common.close')}
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
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={handleConfirmOverwrite}>{t('common.confirm')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}
