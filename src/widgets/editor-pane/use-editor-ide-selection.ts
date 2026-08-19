import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { ideStatusQueryOptions } from '@entities/ide/ide.query'
import { clearIdeSelection, setIdeSelection } from '@entities/ide/ide.ipc'

const IDE_SELECTION_PUSH_DEBOUNCE_MS = 300

type UseEditorIdeSelectionInput = {
    projectId: ProjectId
    path: string
    editor: monaco.editor.IStandaloneCodeEditor | null
}

/**
 * Owns `EditorPane`'s IDE-selection push — while an attached IDE (JetBrains via the `ideStatus`
 * bridge) is running, forwards this editor's debounced cursor/selection range to it, and clears
 * that selection on unmount/editor swap.
 */
export const useEditorIdeSelection = ({ projectId, path, editor }: UseEditorIdeSelectionInput) => {
    const selectionTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

    const { data: ideStatus } = useQuery(ideStatusQueryOptions())

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
}
