import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { AppFileTarget, ProjectId, TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { resolveCodeEditorSettingsProps } from '@shared/lib/code-editor-settings'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { resolveAppFileModelPath } from '@entities/app-file/app-file-model-path'
import { appFileQueryOptions, useWriteAppFile } from '@entities/app-file/app-file.query'
import { applyExternalContent } from '@entities/editor/model-registry'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { CodeEditor } from '@features/editor/code-editor'

const APP_FILE_LANGUAGE_ID = 'json'

type AppFilePaneProps = {
    projectId: ProjectId
    tabId: TabId
    target: AppFileTarget
    initialDirty: boolean
}

/**
 * Editor pane for an `AppFile` tab (`settings.json` or a prompt template override) — a JSON document
 * that lives at an app-owned path the frontend never sees (contract §3.3), read/written through
 * `app_file_read`/`app_file_write` instead of the regular file IPC. Deliberately has no hot-exit
 * mirror (contract §3.3: 1차 제외) and no external-file-watcher reconciliation beyond re-syncing to
 * whatever `app_file_write` actually persisted (relevant for `Settings`, whose write path sanitizes
 * the parsed value before saving — the on-disk text can end up slightly different from what was typed).
 *
 * `initialDirty` — the persisted tab's own `dirty` flag — seeds local `dirty` state instead of
 * always starting `false`: `pane-node-view.tsx` only renders the *active* tab, so switching away
 * from and back to this tab unmounts and remounts this whole component, resetting every local
 * `useState`. The monaco model itself survives that (`entities/editor/model-registry.ts` is a
 * module-level singleton, not React state), but without this seed the freshly-mounted `dirty` would
 * read `false` and the sync effect below would treat the still-unsaved model as stale and overwrite
 * it with the last-synced disk content — silently discarding the edit. `handleEditorMount` mirrors
 * that seed into `draftRef` from the model's live value so `⌘S` works immediately after switching
 * back, even before the user types another character in this new component instance.
 */
export const AppFilePane: FC<AppFilePaneProps> = ({ projectId, tabId, target, initialDirty }) => {
    const draftRef = useRef<string | null>(null)

    const [syncedContent, setSyncedContent] = useState<string | null>(null)
    const [dirty, setDirty] = useState(initialDirty)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)

    const { t } = useTranslation()
    const { data: content, isPending, isError } = useQuery(appFileQueryOptions(target))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: updateSettings } = useUpdateSettings()
    const { mutateAsync: writeAppFile } = useWriteAppFile()

    const modelPath = resolveAppFileModelPath(target)

    if (content !== undefined && syncedContent === null) setSyncedContent(content)
    else if (content !== undefined && !dirty && syncedContent !== null && content !== syncedContent) setSyncedContent(content)

    const handleChange = (value: string) => {
        draftRef.current = value
        if (!dirty) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }
    }

    /**
     * `Settings` failures always surface the fixed `settingsJsonInvalid` message — `app_file_write`'s
     * only failure mode for that target is `parse_settings_json` rejecting malformed JSON before
     * `sanitize` ever runs (sanitize itself cannot fail). `Prompt` has no dedicated locale copy here,
     * so it falls through to {@link describeIpcError}, which resolves `validate_prompt_json`'s own
     * `error.app.promptTemplateInvalidJson` catalog key (matching the existing helper-mediated
     * fallback elsewhere in this codebase — e.g. `plugin-manager.tsx`'s VSIX import catch block).
     */
    const handleSave = async () => {
        const value = draftRef.current
        if (value === null) return
        try {
            await writeAppFile({ target, content: value })
            if (draftRef.current !== value) return
            setDirty(false)
            setTabDirty({ tabId, dirty: false })
        } catch (error) {
            if (target.kind === 'settings') {
                toast.error(t('settings.settingsJsonInvalid'))
                return
            }
            toast.error(describeIpcError(error))
        }
    }

    const handleMinimapToggle = (enabled: boolean) => updateSettings({ ...emptySettingsPatch(), editorMinimap: enabled })

    const handleEditorMount = (nextEditor: monaco.editor.IStandaloneCodeEditor | null) => {
        setEditor(nextEditor)
        if (nextEditor && dirty && draftRef.current === null) draftRef.current = nextEditor.getModel()?.getValue() ?? null
    }

    const aiCompletionConfig = resolveAiInlineCompletionConfig(settings, aiTokenStatus)

    /**
     * `useEffectEvent` so the sync below can read the always-current `editor`/`modelPath` without
     * being a reactive dependency — mirrors `editor-pane.tsx`'s `syncModelOrPickUpExternalEdit`,
     * minus the LSP-model-dirty-tracker branch (no LSP session ever attaches to an `AppFile` model).
     */
    const applySyncedContent = useEffectEvent(() => {
        if (!editor || syncedContent === null) return
        applyExternalContent(modelPath, syncedContent, editor)
    })

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        queueMicrotask(applySyncedContent)
    }, [editor, syncedContent, dirty])

    if (isPending) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {t('editor.openFailed')}
            </div>
        )
    }

    return (
        <CodeEditor
            path={modelPath}
            language={APP_FILE_LANGUAGE_ID}
            value={content}
            readOnly={false}
            largeFile={false}
            {...resolveCodeEditorSettingsProps(settings)}
            formatOnType={false}
            formatOnPaste={false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={() => void handleSave()}
            onCursorLineChange={() => undefined}
            onEditorMount={handleEditorMount}
            onMinimapToggle={handleMinimapToggle}
        />
    )
}
