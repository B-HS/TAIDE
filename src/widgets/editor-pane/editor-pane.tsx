import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Columns2 } from 'lucide-react'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import type { BlameLine, HunkKind, MirrorEntry, ProjectId, TabId } from '@shared/api/bindings'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { monaco } from '@shared/lib/monaco/setup'
import { formatBlameLine } from '@shared/lib/blame-format'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { renderMarkdownToSafeHtml } from '@shared/lib/markdown'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { HOT_EXIT_MIRROR_DEBOUNCE_MS } from '@shared/constants/mirror'
import { QUERY_KEY } from '@shared/constants/query-key'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { fileMirrorsQueryOptions, fileQueryOptions, useSaveFile } from '@entities/file/file.query'
import { clearMirror, mirrorDirty } from '@entities/file/file.ipc'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { getGitBlameRange } from '@entities/git/git.ipc'
import { ideStatusQueryOptions } from '@entities/ide/ide.query'
import { systemOpenPath } from '@entities/system/system.ipc'
import { clearIdeSelection, setIdeSelection } from '@entities/ide/ide.ipc'
import { gitCurrentUserQueryOptions, gitGutterQueryOptions, useDiscardGitHunk } from '@entities/git/git.query'
import { HunkDiscardDialog } from '@features/git/hunk-discard-dialog'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { applyExternalContent } from '@entities/editor/model-registry'
import { registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'
import { consumePendingReveal } from '@entities/editor/reveal-registry'
import type { EditorCursorBlinkingStyle, EditorCursorStyle, EditorRenderWhitespace } from '@features/editor/code-editor'
import { CodeEditor } from '@features/editor/code-editor'
import { BlameFooterBar } from '@features/editor/blame-footer-bar'
import type { ConflictBannerVariant } from '@features/editor/conflict-banner'
import { ConflictBanner } from '@features/editor/conflict-banner'
import { MarkdownPreview } from '@features/editor/markdown-preview'
import { PaneSeparator } from '@features/split/pane-separator'
import { Button } from '@shared/ui/button'
import { useLspSession } from '@widgets/editor-pane/use-lsp-session'

const BLAME_DEBOUNCE_MS = 300
const MARKDOWN_PREVIEW_DEBOUNCE_MS = 200
const IDE_SELECTION_PUSH_DEBOUNCE_MS = 300
const MARKDOWN_LANGUAGE_ID = 'markdown'
const FORMAT_DOCUMENT_ACTION_ID = 'editor.action.formatDocument'
const TOGGLE_PREVIEW_BUTTON_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

const DEFAULT_EDITOR_TAB_SIZE = 4
const DEFAULT_EDITOR_RENDER_WHITESPACE: EditorRenderWhitespace = 'selection'
const DEFAULT_EDITOR_CURSOR_STYLE: EditorCursorStyle = 'line'
const DEFAULT_EDITOR_CURSOR_BLINKING: EditorCursorBlinkingStyle = 'blink'

const GUTTER_CLASS_BY_HUNK_KIND: Record<HunkKind, string> = {
    added: 'taide-gutter-added',
    modified: 'taide-gutter-modified',
    deleted: 'taide-gutter-deleted',
}

type EditorPaneProps = {
    projectId: ProjectId
    tabId: TabId
    path: string
}

export const EditorPane: FC<EditorPaneProps> = ({ projectId, tabId, path }) => {
    const draftRef = useRef<string | null>(null)
    const pendingMirrorRef = useRef(false)
    const mirrorTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const blameTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const previewTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const autoSaveTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const selectionTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const blameRequestSeqRef = useRef(0)
    const blameFooterTextRef = useRef<HTMLSpanElement>(null)

    const [syncedPath, setSyncedPath] = useState(path)
    const [syncedContent, setSyncedContent] = useState<string | null>(null)
    const [dirty, setDirty] = useState(false)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)
    const [cursorLine, setCursorLine] = useState<number | null>(null)
    const [pendingHunk, setPendingHunk] = useState<{ start: number; end: number } | null>(null)
    const [blameLine, setBlameLine] = useState<BlameLine | null>(null)
    const [showMarkdownPreview, setShowMarkdownPreview] = useState(false)
    const [previewSource, setPreviewSource] = useState<string | null>(null)
    const [restoreNotice, setRestoreNotice] = useState<Exclude<ConflictBannerVariant, 'changedOnDisk'> | 'none'>('none')

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: file, isPending, isError, error } = useQuery(fileQueryOptions(path))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { data: ideStatus } = useQuery(ideStatusQueryOptions())
    const { data: gutterHunks } = useQuery(gitGutterQueryOptions({ projectId, path }))
    const { data: mirrors } = useQuery(fileMirrorsQueryOptions(projectId))
    const { mutate: discardHunk } = useDiscardGitHunk(projectId)
    const { data: currentUser } = useQuery(gitCurrentUserQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile()
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: updateSettings } = useUpdateSettings()

    if (path !== syncedPath) {
        setSyncedPath(path)
        setSyncedContent(null)
        setPreviewSource(null)
        setDirty(false)
        setBlameLine(null)
        setRestoreNotice('none')
    } else if (file && syncedContent === null) {
        setSyncedContent(file.content)
    } else if (file && !dirty && syncedContent !== null && file.content !== syncedContent) {
        setSyncedContent(file.content)
        setPreviewSource(null)
    }

    const conflict = dirty && syncedContent !== null && !!file && file.content !== syncedContent
    const isMarkdown = file?.languageId === MARKDOWN_LANGUAGE_ID

    /**
     * Writes to the hot-exit mirror and keeps the `FILE.MIRRORS` query cache in lockstep, instead
     * of leaving it at its `staleTime: Infinity` project-activation snapshot until the next
     * save/view-disk/prune. Without this, revisiting this tab after a same-pane detour to another
     * tab (`EditorPane` has no `key`, so a path switch reuses this instance) would restore from a
     * stale cached entry — either missing entirely (this tab's edits since activation never landed
     * in the cache) or, worse, an *older* mirror than what's already on screen, silently rolling
     * back newer edits. `disk_modified_ms`/`conflict` mirror what Rust would compute for a mirror
     * written against the currently-known disk baseline with nothing external having changed it.
     */
    const persistMirror = async (content: string) => {
        await mirrorDirty({ projectId, path, content, diskModifiedMs: file?.modifiedMs ?? null })
        queryClient.setQueryData(QUERY_KEY.FILE.MIRRORS(projectId), (previous?: MirrorEntry[]) => [
            ...(previous ?? []).filter((entry) => entry.path !== path),
            { path, content, savedAtMs: Date.now(), diskModifiedMs: file?.modifiedMs ?? null, conflict: false },
        ])
    }

    const handleChange = (value: string) => {
        draftRef.current = value
        if (!dirty) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }

        pendingMirrorRef.current = true
        clearTimeout(mirrorTimeoutRef.current)
        mirrorTimeoutRef.current = setTimeout(() => {
            pendingMirrorRef.current = false
            void persistMirror(value).catch(() => undefined)
        }, HOT_EXIT_MIRROR_DEBOUNCE_MS)

        const autoSaveDelayMs = settings?.autoSaveDelayMs ?? 0
        clearTimeout(autoSaveTimeoutRef.current)
        if (autoSaveDelayMs > 0) {
            autoSaveTimeoutRef.current = setTimeout(() => {
                void handleSave()
            }, autoSaveDelayMs)
        }

        if (!isMarkdown) return
        clearTimeout(previewTimeoutRef.current)
        previewTimeoutRef.current = setTimeout(() => setPreviewSource(value), MARKDOWN_PREVIEW_DEBOUNCE_MS)
    }

    const handleSave = async () => {
        const content = draftRef.current
        if (content === null) return

        clearTimeout(autoSaveTimeoutRef.current)

        if (settings?.formatOnSave) {
            const formatAction = editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)
            if (formatAction) await formatAction.run().catch(() => undefined)
        }

        const finalContent = draftRef.current
        if (finalContent === null) return

        saveFile(
            { path, content: finalContent },
            {
                onSuccess: () => {
                    clearTimeout(mirrorTimeoutRef.current)
                    pendingMirrorRef.current = false
                    setDirty(false)
                    setTabDirty({ tabId, dirty: false })
                    setRestoreNotice('none')
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
                },
                onError: (saveError) => toast.error(saveError.message),
            },
        )
    }

    const handleViewDisk = () => {
        if (!file) return

        clearTimeout(mirrorTimeoutRef.current)
        pendingMirrorRef.current = false
        draftRef.current = file.content
        setSyncedContent(file.content)
        setPreviewSource(null)
        setDirty(false)
        setTabDirty({ tabId, dirty: false })
        setRestoreNotice('none')
        void clearMirror({ projectId, path }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(projectId) })
    }

    const handleKeepMine = () => {
        if (file) setSyncedContent(file.content)
        setRestoreNotice('none')
    }

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => {
        setEditor(nextEditor)
        if (nextEditor) registerEditorInstance(tabId, nextEditor)
        else unregisterEditorInstance(tabId)
    }

    useLspSession({
        projectId,
        path,
        languageId: file?.languageId ?? null,
        tier: file?.tier ?? null,
        enabled: !isPending && !isError && file?.tier !== 'refused',
    })

    /**
     * `useEffectEvent` so the restore below can read the always-current `editor`/`path`/`tabId`
     * without being a reactive dependency of the effect that calls it — the effect's own guard
     * (`draftRef.current !== null`) is what decides whether a restore is due, not this event's deps.
     */
    const applyMirrorRestore = useEffectEvent((mirror: MirrorEntry) => {
        if (!editor) return
        applyExternalContent(path, mirror.content, editor)
        draftRef.current = mirror.content
        setDirty(true)
        setTabDirty({ tabId, dirty: true })
        setRestoreNotice(mirror.conflict ? 'mirrorRestoredConflict' : 'mirrorRestored')
    })

    useEffect(() => {
        draftRef.current = null
    }, [path])

    /**
     * Restores unsaved edits from the hot-exit mirror the first time the monaco editor is mounted
     * and nothing has been typed yet for this `path` (a fresh mount or a path switch, never
     * mid-edit — the reset effect above always runs first for a path switch within the same commit).
     * `mirror.conflict` (computed in Rust from `disk_modified_ms` vs the mirror's baseline) decides
     * whether this surfaces as a plain restore notice or a conflict requiring the user to choose.
     */
    useEffect(() => {
        if (!editor || draftRef.current !== null) return
        const mirror = (mirrors ?? []).find((entry) => entry.path === path)
        if (mirror) queueMicrotask(() => applyMirrorRestore(mirror))
    }, [editor, path, mirrors])

    /**
     * Registers a flush callback the hot-exit `CloseRequested` handler (and window blur / a path
     * switch that unmounts this content) can invoke to push the last debounced edit to the mirror
     * immediately, instead of losing up to `HOT_EXIT_MIRROR_DEBOUNCE_MS` of unmirrored edits.
     * `pendingMirrorRef` (not `dirty`) gates the write so a save/view-disk that clears the mirror
     * right before this effect's `file.modifiedMs` dependency changes can't race a stale flush back
     * into existence.
     */
    useEffect(() => {
        const flush = async () => {
            clearTimeout(mirrorTimeoutRef.current)
            if (!pendingMirrorRef.current || draftRef.current === null) return
            pendingMirrorRef.current = false
            await persistMirror(draftRef.current).catch(() => undefined)
        }

        registerMirrorFlush(tabId, flush)
        window.addEventListener('blur', flush)
        return () => {
            window.removeEventListener('blur', flush)
            void flush()
            unregisterMirrorFlush(tabId)
        }
    }, [projectId, path, tabId, file?.modifiedMs])

    useEffect(() => () => clearTimeout(autoSaveTimeoutRef.current), [])

    useEffect(() => () => clearTimeout(previewTimeoutRef.current), [])

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        applyExternalContent(path, syncedContent, editor)
    }, [editor, syncedContent, dirty, path])

    useEffect(() => {
        if (!editor) return
        consumePendingReveal(path, editor)
    }, [editor, path])

    useEffect(() => {
        if (!editor) return

        const decorations = (gutterHunks ?? []).map((hunk) => ({
            range: new monaco.Range(hunk.start, 1, hunk.end, 1),
            options: { linesDecorationsClassName: GUTTER_CLASS_BY_HUNK_KIND[hunk.kind], isWholeLine: true },
        }))
        const collection = editor.createDecorationsCollection(decorations)
        return () => collection.clear()
    }, [editor, gutterHunks])

    useEffect(() => {
        if (!editor) return
        const subscription = editor.onMouseDown((event) => {
            if (event.target.type !== monaco.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) return
            const line = event.target.position?.lineNumber
            if (!line) return
            const hunk = (gutterHunks ?? []).find((candidate) => line >= candidate.start && line <= candidate.end)
            if (!hunk) return
            setPendingHunk({ start: hunk.start, end: hunk.end })
        })
        return () => subscription.dispose()
    }, [editor, gutterHunks])

    useEffect(() => {
        if (!editor || !ideStatus?.running) return

        const subscription = editor.onDidChangeCursorSelection((event) => {
            clearTimeout(selectionTimeoutRef.current)
            selectionTimeoutRef.current = setTimeout(() => {
                const model = editor.getModel()
                if (!model) return
                const range = monacoRangeToLsp(event.selection)
                void setIdeSelection({
                    projectId,
                    path,
                    text: model.getValueInRange(event.selection),
                    startLine: range.start.line,
                    startCharacter: range.start.character,
                    endLine: range.end.line,
                    endCharacter: range.end.character,
                    isEmpty: event.selection.isEmpty(),
                }).catch(() => undefined)
            }, IDE_SELECTION_PUSH_DEBOUNCE_MS)
        })

        return () => {
            subscription.dispose()
            clearTimeout(selectionTimeoutRef.current)
            void clearIdeSelection().catch(() => undefined)
        }
    }, [editor, ideStatus?.running, projectId, path])

    useEffect(() => {
        if (!editor || cursorLine === null) return

        clearTimeout(blameTimeoutRef.current)
        blameTimeoutRef.current = setTimeout(() => {
            const requestSeq = ++blameRequestSeqRef.current
            void getGitBlameRange({ projectId, path, from: cursorLine, to: cursorLine })
                .then((lines) => {
                    if (blameRequestSeqRef.current !== requestSeq) return
                    setBlameLine(lines[0] ?? null)
                })
                .catch(() => {
                    if (blameRequestSeqRef.current !== requestSeq) return
                    setBlameLine(null)
                })
        }, BLAME_DEBOUNCE_MS)

        return () => clearTimeout(blameTimeoutRef.current)
    }, [editor, cursorLine, projectId, path])

    useEffect(() => {
        const node = blameFooterTextRef.current
        if (!node) return

        const model = editor?.getModel() ?? null
        node.textContent = !blameLine || (model && blameLine.line > model.getLineCount()) ? '' : formatBlameLine(blameLine, Date.now(), currentUser)
    }, [editor, blameLine, currentUser])

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
            renderWhitespace={(settings?.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE) as EditorRenderWhitespace}
            bracketPairColorization={settings?.editorBracketPairColorization ?? true}
            fontLigatures={settings?.editorFontLigatures ?? false}
            cursorStyle={(settings?.editorCursorStyle ?? DEFAULT_EDITOR_CURSOR_STYLE) as EditorCursorStyle}
            cursorBlinking={(settings?.editorCursorBlinking ?? DEFAULT_EDITOR_CURSOR_BLINKING) as EditorCursorBlinkingStyle}
            scrollBeyondLastLine={settings?.editorScrollBeyondLastLine ?? true}
            stickyScroll={settings?.editorStickyScrollEnabled ?? true}
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

    const handleConfirmDiscardHunk = () => {
        if (!pendingHunk) return
        discardHunk(
            { projectId, path, hunkStart: pendingHunk.start, hunkEnd: pendingHunk.end },
            { onError: (mutationError) => toast.error(mutationError.message) },
        )
        setPendingHunk(null)
    }

    const bannerVariant: ConflictBannerVariant | 'none' = conflict ? 'changedOnDisk' : restoreNotice

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <HunkDiscardDialog
                startLine={pendingHunk?.start ?? null}
                endLine={pendingHunk?.end ?? null}
                onCancel={() => setPendingHunk(null)}
                onConfirm={handleConfirmDiscardHunk}
            />
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
