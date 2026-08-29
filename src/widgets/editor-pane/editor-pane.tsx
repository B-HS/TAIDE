import type { CSSProperties, FC } from 'react'
import { useEffect, useEffectEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Columns2 } from 'lucide-react'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import type { monaco } from '@shared/lib/monaco/setup'
import { resolveCodeEditorSettingsProps } from '@shared/lib/code-editor-settings'
import { resolveDiffViewSettingsProps } from '@shared/lib/diff-view-settings'
import { resolveEditorConfigIndentProps } from '@shared/lib/editorconfig'
import { requestEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'
import { renderMarkdownToSafeHtml } from '@shared/lib/markdown'
import { consumeExternallyDirtyModel } from '@shared/lib/lsp/model-dirty-tracker'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { useIpcErrorMessage } from '@shared/hooks/use-ipc-error-message'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { fileQueryOptions } from '@entities/file/file.query'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { applyExternalContent } from '@entities/editor/model-registry'
import { consumePendingReveal } from '@entities/editor/reveal-registry'
import { CodeEditor } from '@features/editor/code-editor'
import { BlameFooterBar } from '@features/editor/blame-footer-bar'
import type { ConflictBannerVariant } from '@features/editor/conflict-banner'
import { ConflictBanner } from '@features/editor/conflict-banner'
import { ConflictCompareDialog } from '@features/git/conflict-compare-dialog'
import { ConflictResolutionDialog } from '@features/git/conflict-resolution-dialog'
import { HunkDiscardDialog } from '@features/git/hunk-discard-dialog'
import { MarkdownPreview } from '@features/editor/markdown-preview'
import { PaneSeparator } from '@features/split/pane-separator'
import { Button } from '@shared/ui/button'
import { systemOpenPath } from '@entities/system/system.ipc'
import { resolveEditorStateForRender } from '@widgets/editor-pane/code-editor-visibility'
import { hasChangedOnDiskConflict, syncModelFromDisk } from '@widgets/editor-pane/editor-draft-sync'
import { useEditorLspIntegration } from '@widgets/editor-pane/use-editor-lsp-integration'
import { useEditorFilePersistence } from '@widgets/editor-pane/use-editor-file-persistence'
import { useEditorGitGutterAndConflicts } from '@widgets/editor-pane/use-editor-git-gutter-and-conflicts'
import { useEditorBlame } from '@widgets/editor-pane/use-editor-blame'
import { useEditorMarkdownPreview } from '@widgets/editor-pane/use-editor-markdown-preview'
import { useEditorIdeSelection } from '@widgets/editor-pane/use-editor-ide-selection'
import { useEditorViewState } from '@widgets/editor-pane/use-editor-view-state'
import { BreadcrumbsBar } from '@widgets/editor-pane/breadcrumbs-bar'

const MARKDOWN_LANGUAGE_ID = 'markdown'
const TOGGLE_PREVIEW_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

type EditorPaneProps = {
    projectId: ProjectId
    tabId: TabId
    path: string
}

export const EditorPane: FC<EditorPaneProps> = ({ projectId, tabId, path }) => {
    const [syncedPath, setSyncedPath] = useState(path)
    const [syncedContent, setSyncedContent] = useState<string | null>(null)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)

    const { t } = useTranslation()
    const { data: file, isPending, isError, error } = useQuery(fileQueryOptions(path))
    const openErrorMessage = useIpcErrorMessage(error)
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: updateSettings } = useUpdateSettings()

    const isMarkdown = file?.languageId === MARKDOWN_LANGUAGE_ID

    const { previewTimeoutRef, showMarkdownPreview, setShowMarkdownPreview, previewSource, setPreviewSource } = useEditorMarkdownPreview()

    const { notifyLspSessionsOfSave, runCodeActionsOnSave } = useEditorLspIntegration({
        projectId,
        path,
        languageId: file?.languageId ?? null,
        tier: file?.tier ?? null,
        editor,
        fixAllOnSave: settings?.fixAllOnSave,
        organizeImportsOnSave: settings?.organizeImportsOnSave,
        isPending,
        isError,
        t,
    })

    const {
        dirty,
        isDraftDirty,
        setDirty,
        adoptUnobservedModelEdit,
        restoreNotice,
        setRestoreNotice,
        handleChange,
        handleSave,
        handleViewDisk,
        handleKeepMine,
        settleAfterDiskWrite,
    } = useEditorFilePersistence({
        projectId,
        path,
        tabId,
        file,
        autoSaveDelayMs: settings?.autoSaveDelayMs,
        formatOnSave: settings?.formatOnSave,
        trimTrailingWhitespaceOnSave: settings?.trimTrailingWhitespaceOnSave,
        insertFinalNewlineOnSave: settings?.insertFinalNewlineOnSave,
        isMarkdown,
        editor,
        setSyncedContent,
        setTabDirty,
        notifyLspSessionsOfSave,
        runCodeActionsOnSave,
        previewTimeoutRef,
        setPreviewSource,
        t,
    })

    const { setCursorLine, blameFooterTextRef, setBlameLine, setBlameOverlayEnabled } = useEditorBlame({ projectId, path, editor, t })

    const {
        pendingHunk,
        setPendingHunk,
        handleConfirmDiscardHunk,
        pendingConflict,
        setPendingConflict,
        handleAcceptCurrentChange,
        handleAcceptIncomingChange,
        handleAcceptBothChanges,
        handleCompareConflict,
        compareSides,
        setCompareSides,
    } = useEditorGitGutterAndConflicts({ projectId, path, editor, t, settleAfterDiskWrite })

    if (path !== syncedPath) {
        setSyncedPath(path)
        setSyncedContent(null)
        setPreviewSource(null)
        setDirty(false)
        setBlameLine(null)
        setBlameOverlayEnabled(false)
        setRestoreNotice('none')
        setCompareSides(null)
    } else if (file && syncedContent === null) {
        setSyncedContent(file.content)
    } else if (file && !dirty && syncedContent !== null && file.content !== syncedContent) {
        setSyncedContent(file.content)
        setPreviewSource(null)
    }

    const conflict = hasChangedOnDiskConflict({ isDirty: dirty, syncedContent, diskContent: file?.content ?? null })

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => setEditor(nextEditor)

    /**
     * Before syncing this tab's model to the last-known disk content, checks whether an LSP
     * `WorkspaceEdit` landed on this model while it had no mounted editor watching it (a
     * background tab in another pane — see `model-dirty-tracker.ts`). If so, that edit is the
     * model's current value and must become this tab's dirty draft instead of being silently
     * overwritten by `applyExternalContent`, which cannot otherwise tell "never diverged from
     * disk" apart from "diverged via an edit this component never observed". Deferred to a
     * microtask (matching `applyMirrorRestore` in the persistence hook) since it calls `setState`,
     * which an effect body must not do synchronously.
     *
     * Dirtiness is read through `isDraftDirty()` rather than the `dirty` state this render closed
     * over: the mirror restore that this microtask races is queued in the SAME commit and turns the
     * pane dirty without any re-render in between, so a snapshot read here is stale by construction
     * and destroys the recovered buffer. See {@link syncModelFromDisk}.
     */
    const syncModelOrPickUpExternalEdit = useEffectEvent(() => {
        if (!editor || syncedContent === null) return
        syncModelFromDisk({
            isDraftDirty,
            hasUnobservedModelEdit: () => consumeExternallyDirtyModel(path),
            adoptUnobservedModelEdit: () => {
                const model = editor.getModel()
                if (model) adoptUnobservedModelEdit(() => model.getValue())
            },
            applyDiskContent: () => applyExternalContent(path, syncedContent, editor),
        })
    })

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        queueMicrotask(syncModelOrPickUpExternalEdit)
    }, [editor, syncedContent, dirty, path])

    /**
     * Called before `consumePendingReveal` below (not with the other custom hooks up top) so an
     * explicit navigation into an already-open tab — the one case where both this hook's first-visit
     * restore and a pending reveal could target the same commit — always wins: `useEditorViewState`
     * only ever restores a `tabId` once per `EditorPane` instance, and a reveal only ever targets a
     * tab that's already open (never a first-visit-with-no-persisted-viewState tab), so the two never
     * really collide, but this ordering is what guarantees a reveal's `editor.setPosition` is always
     * the *last* word on the cursor position for this commit regardless.
     */
    useEditorViewState({ projectId, tabId, editor })

    useEffect(() => {
        if (!editor) return
        consumePendingReveal(path, editor)
    }, [editor, path])

    /**
     * Called here rather than up with the other hooks (its own natural position by convention —
     * frontend.md §3.2 groups custom hooks together) so its selection-change subscription registers
     * *after* the `consumePendingReveal` effect above — matching the pre-decomposition order, where a
     * mount-time programmatic reveal (`editor.setPosition`, from a go-to-definition/search-result
     * open) fires before this effect's `onDidChangeCursorSelection` subscription exists, so that
     * reveal's cursor move was never itself pushed to the IDE as a "selection". Registering this
     * subscription earlier would let that reveal's cursor move get caught and debounce-pushed as a
     * user selection, a spurious IPC call the original never made.
     */
    useEditorIdeSelection({ projectId, path, editor })

    useEffect(() => {
        if (!editor) return
        const action = editor.addAction({
            id: 'taide.runSelectedTextInTerminal',
            label: t('terminal.runSelectedText'),
            contextMenuGroupId: '9_terminal',
            run: (targetEditor) => {
                const text = resolveSelectedTextOrCurrentLine(targetEditor)
                if (text !== null) requestEditorPaneCommand({ type: 'run-in-terminal', text, cwd: null })
            },
        })
        return () => action.dispose()
    }, [editor, t])

    /**
     * Render-phase adjustment (same pattern as the `syncedPath` block above — React's
     * documented "adjust state during render") that keeps `editor` state consistent with what
     * this render actually outputs. When the branches below take one of the three early returns
     * that don't render `CodeEditor` (loading placeholder, error, or a refused-tier file) while
     * `editor` still references a live monaco instance, `CodeEditor`'s own unmount cleanup — which
     * nulls this state via `onEditorMount(null)` — only runs in the passive phase, one commit too
     * late. Left uncorrected, several OTHER hooks below that read `editor` state directly rather
     * than through the shared registry (`useEditorGitGutterAndConflicts`, `useEditorBlame`,
     * `useEditorFilePersistence`, `useEditorViewState`, `useEditorIdeSelection`, and the
     * "run selected text in terminal" `addAction` effect further below) would still see that
     * already-disposed instance (`CodeEditor`'s own passive cleanup disposes it earlier in the
     * same destroy pass) for this one extra commit — e.g. `useEditorGitGutterAndConflicts`'s
     * `addAction` effects, whose only guard is `!editor`, would call `.addAction()` on it
     * (harmless by itself, contract §1), while `useEditorFilePersistence`'s
     * `editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)` could throw outright. With no error boundary
     * mounted above `EditorPane`, an uncaught passive-effect error here unmounts the whole React
     * root (`createRootErrorUpdate`) rather than being contained. Nulling `editor` here lands the
     * correction in THIS commit instead, so every hook below that depends on `editor` re-renders
     * with `null` before any of them can commit an effect against the corpse.
     *
     * `CodeEditor`'s own registration into `editor-instance-registry` (crash-class-seal-contract
     * .md §1-1) no longer depends on this adjustment at all — it is keyed off `editorRef.current`
     * inside `CodeEditor` itself, which can only ever be that instance's own live editor or
     * absent, never a stale snapshot borrowed from this component's `editor` state. This
     * adjustment's remaining job is exactly the `editor`-state-direct consumers named above.
     *
     * {@link resolveEditorStateForRender} only covers these three branches
     * (`canRenderCodeEditor` false) — it can't see a commit where `canRenderCodeEditor` stays
     * true throughout but `CodeEditor` still unmounts and remounts because a sibling JSX branch
     * flips element type (the markdown-preview split, previously `<Group>` vs a bare `<div>`).
     * That gap is closed structurally below, by always rendering the same `<Group>`+editor
     * `<Panel>` regardless of preview state. See
     * docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md §1-2, §7,
     * docs/acknowledge/2026-08-20-crash-class-seal-contract.md §1-1.
     */
    const resolvedEditor = resolveEditorStateForRender(editor, isPending, isError, file?.tier)
    if (resolvedEditor !== editor) setEditor(resolvedEditor)

    if (isPending) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>{openErrorMessage}</div>
    }

    if (file.tier === 'refused') {
        return (
            <div className='bg-editor-background text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-2 text-sm'>
                <span>{t('editor.cannotOpen')}</span>
                <span className='text-xs opacity-70'>{t('editor.binaryOrTooLarge')}</span>
                <Button
                    type='button'
                    variant='outline'
                    size='xs'
                    onClick={() => void systemOpenPath(path).catch((error: Error) => toast.error(describeIpcError(error)))}>
                    {t('editor.openExternally')}
                </Button>
            </div>
        )
    }

    const aiCompletionConfig = resolveAiInlineCompletionConfig(settings, aiTokenStatus)

    const codeEditor = (
        <CodeEditor
            path={file.path}
            language={file.languageId}
            value={file.content}
            readOnly={file.readOnly}
            largeFile={file.tier === 'large' || file.tier === 'readOnly'}
            {...resolveCodeEditorSettingsProps(settings)}
            {...resolveEditorConfigIndentProps(file.editorConfig)}
            formatOnType={settings?.editorFormatOnType ?? false}
            formatOnPaste={settings?.editorFormatOnPaste ?? false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={handleSave}
            onCursorLineChange={setCursorLine}
            onEditorMount={handleEditorMount}
            onMinimapToggle={handleMinimapToggle}
            registryTabId={tabId}
        />
    )

    const codeEditorWithBlameFooter = (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='min-h-0 flex-1'>{codeEditor}</div>
            <BlameFooterBar textRef={blameFooterTextRef} />
        </div>
    )

    const bannerVariant: ConflictBannerVariant | 'none' = conflict ? 'changedOnDisk' : restoreNotice

    /**
     * Always the same `<Group>`+editor `<Panel>` regardless of `showMarkdownPreview` — only the
     * preview `<PaneSeparator>`+`<Panel>` are conditional — so `CodeEditor`'s position in the
     * fiber tree never changes shape when preview toggles or a markdown/non-markdown tab switch
     * flips `isMarkdown`. Previously this ternary put `<Group>` (preview on) against a bare
     * `codeEditorWithBlameFooter` `<div>` (preview off) in the same JSX slot; React treats a
     * changed element type at one slot as delete-then-mount, so toggling preview (or switching
     * to/from a cached tab that changes `isMarkdown`) actually unmounted and remounted
     * `CodeEditor` — reproducing, in one commit, the exact registry corpse the render-phase
     * `editor` adjustment above exists to prevent, except `canRenderCodeEditor` stays true here
     * so that adjustment never fires (contract §7). This invariant only holds while `CodeEditor`
     * is never wrapped in another conditional render between here and `<Group>` — adding one
     * would silently reopen the same crash class this section closes.
     *
     * The editor `<Panel>` carries no `defaultSize`. `react-resizable-panels`'s initial-layout
     * pass (`We()` in the installed 4.12.2 dist) gives every panel that DOES declare a
     * `defaultSize` exactly that value, then splits whatever remains evenly across the panels
     * that don't. With the preview panel absent, the editor panel is the only panel in the group
     * and the remainder is the full 100%; with the preview panel present (its own unchanged
     * `defaultSize='50%'`), the remainder is the other 50% — both outcomes match this branch's
     * pre-existing sizing exactly, without depending on `We()`'s output ever needing the
     * separate sum-to-100 renormalization pass (`K()`) a declared `defaultSize='50%'` on a lone
     * panel would have required. Same no-`defaultSize` pattern already used by `editor-area.tsx`'s
     * outer `<Panel id='editor-panes'>`. It also fixes the one frame rendered before the group's
     * own layout effect commits: with no `defaultSize`, that frame's inline style is
     * `flexGrow: 1` (fills the row immediately) rather than a stale 50% `flexBasis`.
     *
     * `overflow` on `<Group>` and the editor `<Panel>` is forced back to `visible` whenever the
     * preview panel is absent, restoring the exact overflow ancestry the previous bare-`<div>`
     * layout had — nearest clipping ancestor `pane-node-view.tsx`'s `overflow-hidden`, one
     * `BreadcrumbsBar` above the editor box. Left at their `react-resizable-panels` defaults,
     * `<Group>`'s root div hard-codes `overflow: hidden` and `<Panel>`'s inner div hard-codes
     * `overflow: auto` (verified against the installed 4.12.2 dist — both values sit ahead of the
     * user-supplied `style` in the same object literal, so passing `style` here does override
     * them, despite `GroupProps.style`'s doc comment claiming `overflow` "cannot be overridden" —
     * that comment does not match this installed version's actual behavior). Left un-overridden,
     * that clips monaco's hover/suggest/parameter-hint widgets (`allowEditorOverflow`, absolutely
     * positioned inside `.monaco-editor`, routinely laid out above the editor's own top edge) at
     * the editor box instead of the pane box — a regression, not an intended effect of this
     * branch always being a `<Group>` now (contract §7.5 regression-1). When the preview panel IS
     * present, `overflow` is left at its library default, unchanged from every prior revision of
     * this branch — markdown-preview-on already used this same `<Group>`+`<Panel>` pair with no
     * style override before this file was touched — since `MarkdownPreview` manages its own
     * internal scrolling (`features/editor/markdown-preview.tsx`'s own `overflow-auto` div)
     * independently of the ancestor `<Panel>`'s overflow.
     */
    const showPreviewPanel = isMarkdown && showMarkdownPreview
    const editorGroupOverflowFix: CSSProperties | undefined = showPreviewPanel ? undefined : { overflow: 'visible' }
    const editorAndPreviewPanels = (
        <Group orientation='horizontal' className='min-h-0 min-w-0 flex-1' style={editorGroupOverflowFix}>
            <Panel id={`${tabId}-editor`} minSize='20%' className='min-h-0 min-w-0' style={editorGroupOverflowFix}>
                {codeEditorWithBlameFooter}
            </Panel>
            {showPreviewPanel && (
                <>
                    <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                    <Panel id={`${tabId}-preview`} defaultSize='50%' minSize='20%' className='min-h-0 min-w-0'>
                        <MarkdownPreview html={renderMarkdownToSafeHtml(previewSource ?? syncedContent ?? file.content)} />
                    </Panel>
                </>
            )}
        </Group>
    )

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <HunkDiscardDialog
                startLine={pendingHunk?.start ?? null}
                endLine={pendingHunk?.end ?? null}
                onCancel={() => setPendingHunk(null)}
                onConfirm={handleConfirmDiscardHunk}
            />
            <ConflictResolutionDialog
                region={pendingConflict}
                onCancel={() => setPendingConflict(null)}
                onAcceptCurrent={handleAcceptCurrentChange}
                onAcceptIncoming={handleAcceptIncomingChange}
                onAcceptBoth={handleAcceptBothChanges}
                onCompare={handleCompareConflict}
            />
            <ConflictCompareDialog
                sides={compareSides}
                languageId={file.languageId}
                diffViewSettings={resolveDiffViewSettingsProps(settings)}
                onOpenChange={(open) => !open && setCompareSides(null)}
            />
            <BreadcrumbsBar projectId={projectId} tabId={tabId} path={path} />
            {file.readOnly && (
                <div className='bg-status-warning/15 text-status-warning shrink-0 px-3 py-1 text-xs'>
                    {t(file.encodingLossy ? 'editor.readOnlyLossyEncoding' : 'editor.readOnlyLargeFile')}
                </div>
            )}
            {bannerVariant !== 'none' && (
                <ConflictBanner
                    variant={bannerVariant}
                    onViewDisk={handleViewDisk}
                    onKeepMine={handleKeepMine}
                    onDismiss={() => setRestoreNotice('none')}
                />
            )}
            {isMarkdown && (
                <div className='border-app-border flex h-8 shrink-0 items-center justify-end border-b px-2'>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <button
                                type='button'
                                aria-pressed={showMarkdownPreview}
                                aria-label={t('editor.toggleMarkdownPreview')}
                                onClick={() => setShowMarkdownPreview((previous) => !previous)}
                                className={TOGGLE_PREVIEW_BUTTON_CLASS}>
                                <Columns2 className='size-4' />
                            </button>
                        </TooltipTrigger>
                        <TooltipContent side='bottom'>{t('editor.toggleMarkdownPreview')}</TooltipContent>
                    </Tooltip>
                </div>
            )}
            {editorAndPreviewPanels}
        </div>
    )
}
