import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { BlameLine, HunkKind, ProjectId, TabId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { formatBlameLine } from '@shared/lib/blame-format'
import { QUERY_KEY } from '@shared/constants/query-key'
import { fileQueryOptions, useSaveFile } from '@entities/file/file.query'
import { mirrorDirty } from '@entities/file/file.ipc'
import { useSetTabDirty } from '@entities/layout/layout.query'
import { getGitBlameRange } from '@entities/git/git.ipc'
import { gitCurrentUserQueryOptions, gitGutterQueryOptions } from '@entities/git/git.query'
import { applyExternalContent } from '@entities/editor/model-registry'
import { CodeEditor } from '@features/editor/code-editor'
import { ConflictBanner } from '@features/editor/conflict-banner'
import { useLspSession } from '@widgets/editor-pane/use-lsp-session'

const DIRTY_MIRROR_DEBOUNCE_MS = 1_500
const BLAME_DEBOUNCE_MS = 300

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
    const mirrorTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const blameTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const blameRequestSeqRef = useRef(0)

    const [syncedPath, setSyncedPath] = useState(path)
    const [syncedContent, setSyncedContent] = useState<string | null>(null)
    const [dirty, setDirty] = useState(false)
    const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null)
    const [cursorLine, setCursorLine] = useState<number | null>(null)
    const [blameLine, setBlameLine] = useState<BlameLine | null>(null)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: file, isPending, isError, error } = useQuery(fileQueryOptions(path))
    const { data: gutterHunks } = useQuery(gitGutterQueryOptions({ projectId, path }))
    const { data: currentUser } = useQuery(gitCurrentUserQueryOptions(projectId))
    const { mutate: saveFile } = useSaveFile()
    const { mutate: setTabDirty } = useSetTabDirty(projectId)

    if (path !== syncedPath) {
        setSyncedPath(path)
        setSyncedContent(null)
        setDirty(false)
        setBlameLine(null)
    } else if (file && syncedContent === null) {
        setSyncedContent(file.content)
    } else if (file && !dirty && syncedContent !== null && file.content !== syncedContent) {
        setSyncedContent(file.content)
    }

    const conflict = dirty && syncedContent !== null && !!file && file.content !== syncedContent

    const handleChange = (value: string) => {
        draftRef.current = value
        if (!dirty) {
            setDirty(true)
            setTabDirty({ tabId, dirty: true })
        }

        clearTimeout(mirrorTimeoutRef.current)
        mirrorTimeoutRef.current = setTimeout(() => {
            void mirrorDirty({ projectId, path, content: value }).catch(() => undefined)
        }, DIRTY_MIRROR_DEBOUNCE_MS)
    }

    const handleSave = () => {
        const content = draftRef.current
        if (content === null) return

        saveFile(
            { path, content },
            {
                onSuccess: () => {
                    clearTimeout(mirrorTimeoutRef.current)
                    setDirty(false)
                    setTabDirty({ tabId, dirty: false })
                    void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
                },
                onError: (saveError) => toast.error(saveError.message),
            },
        )
    }

    const handleViewDisk = () => {
        if (!file) return

        draftRef.current = file.content
        setSyncedContent(file.content)
        setDirty(false)
        setTabDirty({ tabId, dirty: false })
    }

    const handleKeepMine = () => {
        if (file) setSyncedContent(file.content)
    }

    useLspSession({
        projectId,
        path,
        languageId: file?.languageId ?? null,
        tier: file?.tier ?? null,
        enabled: !isPending && !isError && file?.tier !== 'refused',
    })

    useEffect(() => {
        draftRef.current = null
    }, [path])

    useEffect(() => () => clearTimeout(mirrorTimeoutRef.current), [])

    useEffect(() => {
        if (!editor || syncedContent === null || dirty) return
        applyExternalContent(path, syncedContent, editor)
    }, [editor, syncedContent, dirty, path])

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
        if (!editor || !blameLine) return

        const model = editor.getModel()
        if (!model || blameLine.line > model.getLineCount()) return

        const column = model.getLineMaxColumn(blameLine.line)
        const collection = editor.createDecorationsCollection([
            {
                range: new monaco.Range(blameLine.line, column, blameLine.line, column),
                options: {
                    after: { content: `  ${formatBlameLine(blameLine, Date.now(), currentUser)}`, inlineClassName: 'taide-blame-text' },
                    showIfCollapsed: true,
                },
            },
        ])
        return () => collection.clear()
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
            <div className='bg-editor-background text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-1 text-sm'>
                <span>{t('editor.cannotOpen')}</span>
                <span className='text-xs opacity-70'>{t('editor.binaryOrTooLarge')}</span>
            </div>
        )
    }

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            {file.readOnly && (
                <div className='bg-status-warning/15 text-status-warning shrink-0 px-3 py-1 text-xs'>{t('editor.readOnlyLargeFile')}</div>
            )}
            {conflict && <ConflictBanner onViewDisk={handleViewDisk} onKeepMine={handleKeepMine} />}
            <CodeEditor
                path={file.path}
                language={file.languageId}
                value={file.content}
                readOnly={file.readOnly}
                largeFile={file.tier === 'large' || file.tier === 'readOnly'}
                onChange={handleChange}
                onSave={handleSave}
                onCursorLineChange={setCursorLine}
                onEditorMount={setEditor}
            />
        </div>
    )
}
