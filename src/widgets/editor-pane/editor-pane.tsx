import type { FC } from 'react'
import { useEffect, useEffectEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Columns2 } from 'lucide-react'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import type { monaco } from '@shared/lib/monaco/setup'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'
import { renderMarkdownToSafeHtml } from '@shared/lib/markdown'
import { consumeExternallyDirtyModel } from '@shared/lib/lsp/model-dirty-tracker'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { fileQueryOptions } from '@entities/file/file.query'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { applyExternalContent } from '@entities/editor/model-registry'
import { consumePendingReveal } from '@entities/editor/reveal-registry'
import type { EditorCursorBlinkingStyle, EditorCursorStyle, EditorRenderWhitespace } from '@features/editor/code-editor'
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
import { useEditorLspIntegration } from '@widgets/editor-pane/use-editor-lsp-integration'
import { useEditorFilePersistence } from '@widgets/editor-pane/use-editor-file-persistence'
import { useEditorGitGutterAndConflicts } from '@widgets/editor-pane/use-editor-git-gutter-and-conflicts'
import { useEditorBlame } from '@widgets/editor-pane/use-editor-blame'
import { useEditorMarkdownPreview } from '@widgets/editor-pane/use-editor-markdown-preview'
import { useEditorIdeSelection } from '@widgets/editor-pane/use-editor-ide-selection'
import { BreadcrumbsBar } from '@widgets/editor-pane/breadcrumbs-bar'

const MARKDOWN_LANGUAGE_ID = 'markdown'
const TOGGLE_PREVIEW_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

const DEFAULT_EDITOR_TAB_SIZE = 4
const DEFAULT_EDITOR_RENDER_WHITESPACE: EditorRenderWhitespace = 'selection'
const DEFAULT_EDITOR_CURSOR_STYLE: EditorCursorStyle = 'line'
const DEFAULT_EDITOR_CURSOR_BLINKING: EditorCursorBlinkingStyle = 'blink'

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
        draftRef,
        dirty,
        setDirty,
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
        isMarkdown,
        editor,
        setSyncedContent,
        setTabDirty,
        notifyLspSessionsOfSave,
        runCodeActionsOnSave,
        previewTimeoutRef,
        setPreviewSource,
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

    const conflict = dirty && syncedContent !== null && !!file && file.content !== syncedContent

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => setEditor(nextEditor)

    /**
     * Registers the mounted monaco instance under `tabId` for the shared editor-instance registry
     * (breadcrumbs, the status bar, and `editor-area`'s own lookups all read it). Keyed by
     * `[tabId, editor]` rather than done once inside `handleEditorMount` — `CodeEditor`'s own
     * mount effect has an empty dependency array (it swaps buffers via `setModel`, never
     * remounting monaco), and this pane has no `key` either, so `handleEditorMount` fires exactly
     * once for the pane's whole lifetime while the same instance goes on to serve every tab the
     * user switches to in this pane. Without this effect, the registry stayed pinned to whichever
     * tab happened to be active at first mount; every later tab switch left it stale, and
     * `getEditorInstance` for the now-active tab returned nothing (or a leftover instance for a
     * tab that no longer owns it). Re-running on every `tabId` change re-keys the same live
     * instance instead.
     */
    useEffect(() => {
        if (!editor) return
        registerEditorInstance(tabId, editor)
        return () => unregisterEditorInstance(tabId)
    }, [tabId, editor])

    /**
     * Before syncing this tab's model to the last-known disk content, checks whether an LSP
     * `WorkspaceEdit` landed on this model while it had no mounted editor watching it (a
     * background tab in another pane — see `model-dirty-tracker.ts`). If so, that edit is the
     * model's current value and must become this tab's dirty draft instead of being silently
     * overwritten by `applyExternalContent`, which cannot otherwise tell "never diverged from
     * disk" apart from "diverged via an edit this component never observed". Deferred to a
     * microtask (matching `applyMirrorRestore` in the persistence hook) since it calls `setState`,
     * which an effect body must not do synchronously.
     */
    const syncModelOrPickUpExternalEdit = useEffectEvent(() => {
        if (!editor || syncedContent === null || dirty) return
        if (consumeExternallyDirtyModel(path)) {
            const model = editor.getModel()
            if (!model) return
            draftRef.current = model.getValue()
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
            return
        }
        applyExternalContent(path, syncedContent, editor)
    })

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        queueMicrotask(syncModelOrPickUpExternalEdit)
    }, [editor, syncedContent, dirty, path])

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

    if (isPending) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {error instanceof Error ? error.message : t('editor.openFailed')}
            </div>
        )
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
                    onClick={() => void systemOpenPath(path).catch((error: Error) => toast.error(error.message))}>
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
            fontFamily={buildMonospaceFontStack(settings?.editorFontFamily ?? null)}
            fontSize={settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE}
            minimap={settings?.editorMinimap ?? true}
            wordWrap={settings?.editorWordWrap ?? false}
            lineNumbers={settings?.editorLineNumbers ?? true}
            tabSize={settings?.editorTabSize ?? DEFAULT_EDITOR_TAB_SIZE}
            insertSpaces={settings?.editorInsertSpaces ?? true}
            detectIndentation={settings?.editorDetectIndentation ?? true}
            renderWhitespace={settings?.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE}
            bracketPairColorization={settings?.editorBracketPairColorization ?? true}
            fontLigatures={settings?.editorFontLigatures ?? false}
            cursorStyle={settings?.editorCursorStyle ?? DEFAULT_EDITOR_CURSOR_STYLE}
            cursorBlinking={settings?.editorCursorBlinking ?? DEFAULT_EDITOR_CURSOR_BLINKING}
            scrollBeyondLastLine={settings?.editorScrollBeyondLastLine ?? true}
            stickyScroll={settings?.editorStickyScrollEnabled ?? true}
            formatOnType={settings?.editorFormatOnType ?? false}
            formatOnPaste={settings?.editorFormatOnPaste ?? false}
            aiAutoTabEnabled={settings?.aiAutoTabEnabled ?? false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={handleSave}
            onCursorLineChange={setCursorLine}
            onEditorMount={handleEditorMount}
            onMinimapToggle={handleMinimapToggle}
        />
    )

    const codeEditorWithBlameFooter = (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='min-h-0 flex-1'>{codeEditor}</div>
            <BlameFooterBar textRef={blameFooterTextRef} />
        </div>
    )

    const bannerVariant: ConflictBannerVariant | 'none' = conflict ? 'changedOnDisk' : restoreNotice

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
            <ConflictCompareDialog sides={compareSides} languageId={file.languageId} onOpenChange={(open) => !open && setCompareSides(null)} />
            <BreadcrumbsBar projectId={projectId} tabId={tabId} path={path} />
            {file.readOnly && (
                <div className='bg-status-warning/15 text-status-warning shrink-0 px-3 py-1 text-xs'>{t('editor.readOnlyLargeFile')}</div>
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
            {isMarkdown && showMarkdownPreview ? (
                <Group orientation='horizontal' className='min-h-0 min-w-0 flex-1'>
                    <Panel id={`${tabId}-editor`} defaultSize='50%' minSize='20%' className='min-h-0 min-w-0'>
                        {codeEditorWithBlameFooter}
                    </Panel>
                    <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                    <Panel id={`${tabId}-preview`} defaultSize='50%' minSize='20%' className='min-h-0 min-w-0'>
                        <MarkdownPreview html={renderMarkdownToSafeHtml(previewSource ?? syncedContent ?? file.content)} />
                    </Panel>
                </Group>
            ) : (
                codeEditorWithBlameFooter
            )}
        </div>
    )
}
