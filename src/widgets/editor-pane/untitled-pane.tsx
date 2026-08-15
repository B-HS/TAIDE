import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { save } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import type { ProjectId, TabId, UntitledMirrorEntry } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { HOT_EXIT_MIRROR_DEBOUNCE_MS } from '@shared/constants/mirror'
import { QUERY_KEY } from '@shared/constants/query-key'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { clearUntitledMirror, mirrorUntitled } from '@entities/file/file.ipc'
import { untitledMirrorsQueryOptions, useSaveFile } from '@entities/file/file.query'
import { useConvertUntitledTab, useSetTabDirty } from '@entities/layout/layout.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { applyExternalContent, disposeModel, toUntitledModelPath } from '@entities/editor/model-registry'
import { registerMirrorFlush, unregisterMirrorFlush } from '@entities/editor/mirror-flush-registry'
import { dropUntitledContent, getUntitledContent, setUntitledContent } from '@entities/editor/untitled-registry'
import type { EditorCursorBlinkingStyle, EditorCursorStyle, EditorRenderWhitespace } from '@features/editor/code-editor'
import { CodeEditor } from '@features/editor/code-editor'

const UNTITLED_LANGUAGE_ID = 'plaintext'
const DEFAULT_EDITOR_TAB_SIZE = 4
const DEFAULT_EDITOR_RENDER_WHITESPACE: EditorRenderWhitespace = 'selection'
const DEFAULT_EDITOR_CURSOR_STYLE: EditorCursorStyle = 'line'
const DEFAULT_EDITOR_CURSOR_BLINKING: EditorCursorBlinkingStyle = 'blink'

type UntitledPaneProps = {
    projectId: ProjectId
    tabId: TabId
    index: number
}

export const UntitledPane: FC<UntitledPaneProps> = ({ projectId, tabId, index }) => {
    const seedContent = getUntitledContent(projectId, tabId)
    const restoredFromMirrorRef = useRef(seedContent !== null)
    const pendingMirrorRef = useRef(false)
    const mirrorTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const saveEpochRef = useRef(0)

    const draftRef = useRef(seedContent ?? '')

    const [initialContent] = useState(() => seedContent ?? '')
    const [dirty, setDirty] = useState(false)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { data: untitledMirrors } = useQuery(untitledMirrorsQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile()
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: convertUntitled } = useConvertUntitledTab(projectId)
    const { mutate: updateSettings } = useUpdateSettings()

    const untitledPath = toUntitledModelPath(tabId)

    /**
     * Writes to the untitled hot-exit mirror and keeps the `FILE.UNTITLED_MIRRORS` query cache in
     * lockstep, instead of leaving it at its `staleTime: Infinity` project-activation snapshot until
     * the next convert-to-file/prune. See the equivalent `persistMirror` in `editor-pane.tsx` for why
     * an unsynced cache lets a same-pane tab detour restore a stale (or entirely missing) mirror.
     *
     * `epoch` guards the same resurrection-after-conversion race `editor-pane.tsx`'s `persistMirror`
     * guards for regular files: `handleConvertSuccess` (below) already clears the untitled mirror the
     * moment `handleSaveAs` finishes, so a write scheduled before that point must not be trusted to
     * still apply once it resolves — checked both before the IPC call and after it returns, reverting
     * (`clearUntitledMirror`) rather than trusting a write that raced past a completed conversion.
     * Returns whether the write committed, so `handleChange`'s debounce timer knows whether it's safe
     * to clear `pendingMirrorRef`.
     */
    const persistMirror = async (content: string, epoch: number) => {
        if (epoch !== saveEpochRef.current) return false
        await mirrorUntitled({ projectId, tabId, content })
        if (epoch !== saveEpochRef.current) {
            void clearUntitledMirror({ projectId, tabId }).catch(() => undefined)
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId) })
            return false
        }
        queryClient.setQueryData(QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId), (previous?: UntitledMirrorEntry[]) => [
            ...(previous ?? []).filter((entry) => entry.tabId !== tabId),
            { tabId, content, savedAtMs: Date.now() },
        ])
        return true
    }

    const handleChange = (value: string) => {
        draftRef.current = value
        setUntitledContent(projectId, tabId, value)
        if (!dirty) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }

        pendingMirrorRef.current = true
        clearTimeout(mirrorTimeoutRef.current)
        const scheduledEpoch = saveEpochRef.current
        mirrorTimeoutRef.current = setTimeout(() => {
            void persistMirror(value, scheduledEpoch)
                .then((committed) => {
                    if (committed) pendingMirrorRef.current = false
                })
                .catch(() => undefined)
        }, HOT_EXIT_MIRROR_DEBOUNCE_MS)
    }

    const handleConvertSuccess = () => {
        saveEpochRef.current += 1
        clearTimeout(mirrorTimeoutRef.current)
        pendingMirrorRef.current = false
        dropUntitledContent(projectId, tabId)
        disposeModel(untitledPath)
        void clearUntitledMirror({ projectId, tabId }).catch(() => undefined)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId) })
    }

    const handleSaveAs = async () => {
        const defaultPath = project ? `${project.root}/Untitled-${index}` : undefined
        const selected = await save({ defaultPath, title: t('tab.saveAsTitle') })
        if (!selected) return

        saveFile(
            { path: selected, content: draftRef.current },
            {
                onSuccess: () =>
                    convertUntitled({ tabId, path: selected }, { onSuccess: handleConvertSuccess, onError: (error) => toast.error(error.message) }),
                onError: (error) => toast.error(error.message),
            },
        )
    }

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => setEditor(nextEditor)

    const aiCompletionConfig = resolveAiInlineCompletionConfig(settings, aiTokenStatus)

    useEffect(() => {
        setUntitledContent(projectId, tabId, draftRef.current)
    }, [projectId, tabId])

    /**
     * `useEffectEvent` so the restore below can read the always-current `editor`/`untitledPath`/
     * `tabId` without being a reactive dependency of the effect that calls it.
     */
    const applyMirrorRestore = useEffectEvent((mirror: UntitledMirrorEntry) => {
        if (!editor) return
        applyExternalContent(untitledPath, mirror.content, editor)
        draftRef.current = mirror.content
        setUntitledContent(projectId, tabId, mirror.content)
        setDirty(true)
        setTabDirty({ tabId, dirty: true })
    })

    /**
     * Restores unsaved content from the hot-exit untitled mirror once, the first time the monaco
     * editor is mounted and no in-memory registry content already existed at mount (a fresh session
     * after an app restart). `restoredFromMirrorRef` guards against re-applying on later refetches.
     */
    useEffect(() => {
        if (!editor || restoredFromMirrorRef.current) return
        const mirror = (untitledMirrors ?? []).find((entry) => entry.tabId === tabId)
        if (!mirror) return
        restoredFromMirrorRef.current = true
        queueMicrotask(() => applyMirrorRestore(mirror))
    }, [editor, untitledMirrors, tabId])

    useEffect(() => {
        const flush = async () => {
            clearTimeout(mirrorTimeoutRef.current)
            if (!pendingMirrorRef.current) return
            const committed = await persistMirror(draftRef.current, saveEpochRef.current).catch(() => false)
            if (committed) pendingMirrorRef.current = false
        }

        registerMirrorFlush(tabId, flush)
        window.addEventListener('blur', flush)
        return () => {
            window.removeEventListener('blur', flush)
            void flush()
            unregisterMirrorFlush(tabId)
        }
    }, [projectId, tabId])

    return (
        <CodeEditor
            path={untitledPath}
            language={UNTITLED_LANGUAGE_ID}
            value={initialContent}
            readOnly={false}
            largeFile={false}
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
            formatOnType={false}
            formatOnPaste={false}
            aiAutoTabEnabled={settings?.aiAutoTabEnabled ?? false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={() => void handleSaveAs()}
            onCursorLineChange={() => undefined}
            onEditorMount={handleEditorMount}
            onMinimapToggle={handleMinimapToggle}
        />
    )
}
