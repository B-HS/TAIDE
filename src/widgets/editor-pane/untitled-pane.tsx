import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { save } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { QUERY_KEY } from '@shared/constants/query-key'
import { aiTokenStatusQueryOptions } from '@entities/ai/ai.query'
import { useSaveFile } from '@entities/file/file.query'
import { useConvertUntitledTab, useSetTabDirty } from '@entities/layout/layout.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { disposeModel, toUntitledModelPath } from '@entities/editor/model-registry'
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
    const seedContent = getUntitledContent(projectId, tabId) ?? ''

    const draftRef = useRef(seedContent)

    const [initialContent] = useState(() => seedContent)
    const [dirty, setDirty] = useState(false)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { mutate: saveFile } = useSaveFile()
    const { mutate: setTabDirty } = useSetTabDirty(projectId)
    const { mutate: convertUntitled } = useConvertUntitledTab(projectId)
    const { mutate: updateSettings } = useUpdateSettings()

    const untitledPath = toUntitledModelPath(tabId)

    const handleChange = (value: string) => {
        draftRef.current = value
        setUntitledContent(projectId, tabId, value)
        if (!dirty) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }
    }

    const handleConvertSuccess = () => {
        dropUntitledContent(projectId, tabId)
        disposeModel(untitledPath)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
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

    const aiCompletionConfig = resolveAiInlineCompletionConfig(settings, aiTokenStatus)

    useEffect(() => {
        setUntitledContent(projectId, tabId, draftRef.current)
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
            aiAutoTabEnabled={settings?.aiAutoTabEnabled ?? false}
            aiCompletionConfig={aiCompletionConfig}
            onChange={handleChange}
            onSave={() => void handleSaveAs()}
            onCursorLineChange={() => undefined}
            onMinimapToggle={handleMinimapToggle}
        />
    )
}
