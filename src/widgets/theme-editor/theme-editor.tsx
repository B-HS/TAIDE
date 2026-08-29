import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ThemeSummary, ThemeType } from '@shared/api/bindings'
import { themeQueryOptions, useDeleteTheme, useSaveTheme, useThemePreview } from '@entities/theme/theme.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { builtinThemeIdForType, resolveThemeIdAfterDelete } from '@entities/theme/theme-selection'
import { COLOR_NAMESPACES, SYNTAX_TOKENS, TERMINAL_TOKENS, colorTokenKey } from '@entities/theme/theme-tokens'
import {
    buildThemeFromDraft,
    countChangedTokens,
    createThemeDraft,
    generateUniqueThemeId,
    isColorTokenChanged,
    isSyntaxTokenChanged,
    isTerminalTokenChanged,
    isThemeDraftValid,
    renameThemeDraft,
    resolveThemeDraftMetadata,
    resetColorToken,
    resetSyntaxToken,
    resetTerminalToken,
    serializeThemeDraftEdits,
    setColorToken,
    setSyntaxToken,
    setTerminalToken,
    toThemeValues,
    type ThemeDraft,
} from '@shared/lib/theme-draft'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { SettingsSection } from '@features/settings/settings-section'
import { ColorTokenRow } from '@features/theme/color-token-row'
import { SyntaxTokenRow } from '@features/theme/syntax-token-row'
import { ThemeLivePreview } from '@features/theme/theme-live-preview'
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
import { Button } from '@shared/ui/button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

type ThemeEditorProps = {
    sourceThemeId: string
    mode: 'create' | 'edit'
    themes: ThemeSummary[]
    onClose: () => void
}

const resolveBaseThemeId = (sourceThemeId: string, themes: ThemeSummary[], type: ThemeType) =>
    themes.find((theme) => theme.id === sourceThemeId)?.builtin ? sourceThemeId : builtinThemeIdForType(type)

export const ThemeEditor: FC<ThemeEditorProps> = ({ sourceThemeId, mode, themes, onClose }) => {
    const { t } = useTranslation()

    const [draft, setDraft] = useState<ThemeDraft | null>(null)
    const [loadedDraftSignature, setLoadedDraftSignature] = useState<string | null>(null)
    const [syncedSourceId, setSyncedSourceId] = useState(sourceThemeId)
    const [search, setSearch] = useState('')
    const [deleteOpen, setDeleteOpen] = useState(false)
    const [discardOpen, setDiscardOpen] = useState(false)

    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: sourceResolved } = useQuery(themeQueryOptions(sourceThemeId))
    const baseThemeId = sourceResolved ? resolveBaseThemeId(sourceThemeId, themes, sourceResolved.type) : null
    const { data: baseResolved } = useQuery({ ...themeQueryOptions(baseThemeId ?? ''), enabled: Boolean(baseThemeId) })
    const { mutate: saveThemeMutate, isPending: isSaving } = useSaveTheme()
    const { mutate: deleteThemeMutate, isPending: isDeleting } = useDeleteTheme()
    const { mutate: setThemeIdMutate, isPending: isSwitchingTheme } = useSetThemeId()
    const { mutate: updateSettingsMutate, isPending: isUpdatingSettings } = useUpdateSettings()
    const { setPreview, clearPreview } = useThemePreview()

    if (sourceThemeId !== syncedSourceId) {
        setSyncedSourceId(sourceThemeId)
        setDraft(null)
        setLoadedDraftSignature(null)
    } else if (!draft && sourceResolved && baseResolved && baseThemeId) {
        const loadedDraft = createThemeDraft({
            id:
                mode === 'create'
                    ? generateUniqueThemeId(
                          sourceResolved.name,
                          themes.map((theme) => theme.id),
                      )
                    : sourceThemeId,
            name: mode === 'create' ? t('themeEditor.duplicateNameTemplate', { name: sourceResolved.name }) : sourceResolved.name,
            themeType: sourceResolved.type,
            extendsId: baseThemeId,
            base: toThemeValues(baseResolved),
            initial: toThemeValues(sourceResolved),
            metadata: resolveThemeDraftMetadata(sourceResolved, baseResolved),
        })
        setDraft(loadedDraft)
        setLoadedDraftSignature(serializeThemeDraftEdits(loadedDraft))
    }

    const normalizedQuery = search.trim().toLowerCase()
    const matchesQuery = (label: string) => normalizedQuery.length === 0 || label.toLowerCase().includes(normalizedQuery)

    /**
     * Warns only once the draft diverges from what was loaded, in both modes. A `create` draft has
     * never been written anywhere, but a *duplicate nobody edited* is byte-for-byte the theme it was
     * duplicated from — discarding it costs nothing, so treating "opened Duplicate and changed my
     * mind" as unsaved work put a confirmation dialog in front of every cancelled duplicate. See
     * `serializeThemeDraftEdits` for why the live preview makes this loss invisible.
     */
    const hasUnsavedChanges = Boolean(draft) && loadedDraftSignature !== (draft ? serializeThemeDraftEdits(draft) : null)

    const requestClose = () => {
        if (hasUnsavedChanges) {
            setDiscardOpen(true)
            return
        }
        onClose()
    }

    const handleSave = () => {
        if (!draft) return
        saveThemeMutate(buildThemeFromDraft(draft), { onSuccess: onClose, onError: (error) => toast.error(describeIpcError(error)) })
    }

    /**
     * Points `settings.themeId` at the same-type builtin *before* deleting, whenever it is the theme
     * being deleted — see `resolveThemeIdAfterDelete` for why a dangling id leaves the app with no
     * theme at all (audit §4-B B5). The delete is chained on that write's success so a failure never
     * strands the setting on a theme file that is about to disappear.
     *
     * `settings_set_theme` is the right command for an active theme (it also emits `ThemeChanged`,
     * which is what makes other windows re-resolve), but it turns `followSystemTheme` off as a
     * deliberate side effect of an explicit pick. With that flag on, the stored id is not what is
     * being displayed and this write is pure bookkeeping — repairing it must not silently take the
     * user off system-follow — so the patch command is used instead.
     */
    const handleDelete = () => {
        const deleteTheme = () => deleteThemeMutate(sourceThemeId, { onSuccess: onClose, onError: (error) => toast.error(describeIpcError(error)) })
        const fallbackThemeId = draft
            ? resolveThemeIdAfterDelete({ deletedThemeId: sourceThemeId, deletedThemeType: draft.themeType, activeThemeId: settings?.themeId })
            : null
        if (!fallbackThemeId) {
            deleteTheme()
            return
        }
        if (settings?.followSystemTheme) {
            updateSettingsMutate({ ...emptySettingsPatch(), themeId: fallbackThemeId }, { onSuccess: deleteTheme })
            return
        }
        setThemeIdMutate(fallbackThemeId, { onSuccess: deleteTheme })
    }

    useEffect(() => {
        if (draft) setPreview(draft)
    }, [draft, setPreview])
    useEffect(() => clearPreview, [clearPreview])

    if (!draft) return <div className='bg-app-background h-full w-full' />

    const changedCount = countChangedTokens(draft)
    const syntaxRows = SYNTAX_TOKENS.filter((token) => matchesQuery(token))
    const terminalRows = TERMINAL_TOKENS.filter((token) => matchesQuery(token))

    return (
        <div className='bg-app-background text-app-foreground flex h-full w-full flex-col overflow-hidden'>
            <div className='border-app-border flex items-center justify-between gap-4 border-b px-6 py-4'>
                <div className='flex items-center gap-3'>
                    <Button variant='ghost' size='sm' onClick={requestClose}>
                        {t('themeEditor.backToSettings')}
                    </Button>
                    <input
                        value={draft.name}
                        onChange={(event) => setDraft(renameThemeDraft(draft, event.currentTarget.value))}
                        placeholder={t('themeEditor.themeNamePlaceholder')}
                        aria-label={t('themeEditor.themeNamePlaceholder')}
                        className='bg-panel-input-background border-panel-input-border text-app-foreground rounded-sm border px-2 py-1 text-sm font-medium'
                    />
                    <span className='text-app-sidebar-icon-default text-xs'>{t('themeEditor.changedCount', { count: changedCount })}</span>
                </div>
                <div className='flex items-center gap-2'>
                    {mode === 'edit' && (
                        <Button
                            variant='outline'
                            size='sm'
                            onClick={() => setDeleteOpen(true)}
                            disabled={isDeleting || isSwitchingTheme || isUpdatingSettings}>
                            {t('themeEditor.deleteTheme')}
                        </Button>
                    )}
                    <Button size='sm' onClick={handleSave} disabled={isSaving || !isThemeDraftValid(draft)}>
                        {t('themeEditor.save')}
                    </Button>
                </div>
            </div>

            <div className='flex min-h-0 flex-1'>
                <ScrollContainer className='min-w-0 flex-1' viewportClassName='px-6 py-4'>
                    <input
                        value={search}
                        onChange={(event) => setSearch(event.currentTarget.value)}
                        placeholder={t('themeEditor.searchTokensPlaceholder')}
                        aria-label={t('themeEditor.searchTokensAriaLabel')}
                        className='bg-panel-input-background border-panel-input-border text-app-foreground mb-4 w-full rounded-sm border px-2 py-1 text-xs'
                    />

                    <div className='flex flex-col gap-4'>
                        {COLOR_NAMESPACES.map((namespace) => {
                            const rows = namespace.tokens
                                .map((token) => ({ token, key: colorTokenKey(namespace.id, token) }))
                                .filter(({ token }) => matchesQuery(namespace.id) || matchesQuery(token))
                            if (rows.length === 0) return null
                            return (
                                <SettingsSection
                                    key={namespace.id}
                                    id={`theme-editor-ns-${namespace.id}`}
                                    title={t(`themeEditor.ns.${namespace.id}`)}>
                                    {rows.map(({ token, key }) => (
                                        <ColorTokenRow
                                            key={key}
                                            label={token}
                                            value={draft.current.colors[key]}
                                            changed={isColorTokenChanged(draft, key)}
                                            onChange={(value) => setDraft(setColorToken(draft, key, value))}
                                            onReset={() => setDraft(resetColorToken(draft, key))}
                                        />
                                    ))}
                                </SettingsSection>
                            )
                        })}

                        {syntaxRows.length > 0 && (
                            <SettingsSection id='theme-editor-syntax' title={t('themeEditor.syntaxSectionTitle')}>
                                {syntaxRows.map((token) => (
                                    <SyntaxTokenRow
                                        key={token}
                                        label={token}
                                        style={draft.current.syntax[token]}
                                        changed={isSyntaxTokenChanged(draft, token)}
                                        onChange={(patch) => setDraft(setSyntaxToken(draft, token, patch))}
                                        onReset={() => setDraft(resetSyntaxToken(draft, token))}
                                    />
                                ))}
                            </SettingsSection>
                        )}

                        {terminalRows.length > 0 && (
                            <SettingsSection id='theme-editor-terminal' title={t('themeEditor.terminalSectionTitle')}>
                                {terminalRows.map((token) => (
                                    <ColorTokenRow
                                        key={token}
                                        label={token}
                                        value={draft.current.terminal[token]}
                                        changed={isTerminalTokenChanged(draft, token)}
                                        onChange={(value) => setDraft(setTerminalToken(draft, token, value))}
                                        onReset={() => setDraft(resetTerminalToken(draft, token))}
                                    />
                                ))}
                            </SettingsSection>
                        )}
                    </div>
                </ScrollContainer>

                <ScrollContainer className='border-app-border w-96 shrink-0 border-l' viewportClassName='px-4 py-4'>
                    <div className='text-app-sidebar-icon-default mb-2 text-xs font-medium'>{t('themeEditor.previewTitle')}</div>
                    <div className='text-app-sidebar-icon-default mb-3 text-xs'>{t('themeEditor.livePreviewHint')}</div>
                    <ThemeLivePreview values={draft.current} />
                </ScrollContainer>
            </div>

            <AlertDialog open={discardOpen} onOpenChange={setDiscardOpen}>
                <AlertDialogContent size='sm'>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('common.unsavedChangesTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>{t('common.unsavedChangesDescription')}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={onClose}>
                            {t('common.discardChanges')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('themeEditor.deleteConfirmTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>{t('themeEditor.deleteConfirmDescription', { name: draft.name })}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={handleDelete}>
                            {t('themeEditor.deleteTheme')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    )
}
