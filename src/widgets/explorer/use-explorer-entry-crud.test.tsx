import { describe, expect, test } from 'bun:test'
import { i18next } from '@shared/i18n/i18n'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { act, renderHookWithProviders } from '@shared/testing/render'
import { resolveTargetDir } from '@widgets/explorer/explorer-path'
import { useExplorerEntryCrud } from '@widgets/explorer/use-explorer-entry-crud'

const PROJECT_ID = 'project-1'
const PROJECT_ROOT = '/project'

const buildRow = (path: string, kind: FileTreeRow['kind'], expanded = true): FileTreeRow => ({
    id: path,
    path,
    name: path.slice(path.lastIndexOf('/') + 1),
    depth: 0,
    kind,
    expanded,
    gitStatus: null,
})

const rejectLikeUnusedMutation = () => Promise.reject(new Error('not expected in this test'))

type CrudInput = Parameters<typeof useExplorerEntryCrud>[0]

/**
 * `targetDirFor` and `rows` are wired exactly as `explorer-container.tsx` wires them, because the
 * defect this pins lives in that pair: the selection is resolved against the page the tree currently
 * holds, so a row that was deleted a moment ago can no longer aim a create at its own path.
 */
const renderCrud = ({ rows, selectedRow }: { rows: FileTreeRow[]; selectedRow: FileTreeRow | null }) => {
    const toggledPaths: string[] = []

    const rendered = renderHookWithProviders(() =>
        useExplorerEntryCrud({
            projectId: PROJECT_ID,
            projectRoot: PROJECT_ROOT,
            rows,
            selectedRow,
            targetDirFor: (row) => resolveTargetDir(row, rows, PROJECT_ROOT),
            openFileTab: () => {},
            notifyError: () => {},
            setSelectPathRequest: () => {},
            toggleNodeAsync: ((input: { projectId: string; path: string }) => {
                toggledPaths.push(input.path)
                return Promise.resolve({ rows: [], total: 0 })
            }) as CrudInput['toggleNodeAsync'],
            createEntry: rejectLikeUnusedMutation as CrudInput['createEntry'],
            refreshTreeDir: rejectLikeUnusedMutation as CrudInput['refreshTreeDir'],
            revealTreeNode: rejectLikeUnusedMutation as CrudInput['revealTreeNode'],
            renameEntryAsync: rejectLikeUnusedMutation as CrudInput['renameEntryAsync'],
            deleteEntryAsync: rejectLikeUnusedMutation as CrudInput['deleteEntryAsync'],
            t: i18next.t,
        }),
    )

    return { ...rendered, toggledPaths }
}

describe('useExplorerEntryCrud.startDraft', () => {
    test('선택된 디렉터리 안에 초안을 연다', async () => {
        const dir = buildRow('/project/src', 'directory')
        const { result } = renderCrud({ rows: [dir], selectedRow: dir })

        await act(async () => void (await result.current.startDraft('file')))

        expect(result.current.draft).toEqual({ kind: 'file', parentDir: '/project/src' })
    })

    /**
     * The delete path leaves the selection pointing at a directory the tree no longer lists, and the
     * create used to be sent there — `create_dir_all` then rebuilt the directory the user had just
     * deleted, with the new file inside it (d-66 #13).
     */
    test('삭제돼 트리에서 사라진 디렉터리가 선택돼 있으면 초안은 프로젝트 루트에 열린다', async () => {
        const deletedDir = buildRow('/project/src/utils', 'directory')
        const { result } = renderCrud({ rows: [], selectedRow: deletedDir })

        await act(async () => void (await result.current.startDraft('file')))

        expect(result.current.draft).toEqual({ kind: 'file', parentDir: PROJECT_ROOT })
    })

    /** The tree's rows start at the root's children, so the root is the one target that legitimately has no row — refusing it would break "New File" with nothing selected. */
    test('프로젝트 루트는 행이 없어도 정상 대상이다', async () => {
        const { result } = renderCrud({ rows: [buildRow('/project/a.ts', 'file')], selectedRow: null })

        await act(async () => void (await result.current.startDraft('directory')))

        expect(result.current.draft).toEqual({ kind: 'directory', parentDir: PROJECT_ROOT })
    })

    test('접힌 디렉터리는 먼저 펼친 뒤 초안을 연다', async () => {
        const dir = buildRow('/project/src', 'directory', false)
        const { result, toggledPaths } = renderCrud({ rows: [dir], selectedRow: dir })

        await act(async () => void (await result.current.startDraft('file')))

        expect(toggledPaths).toEqual([dir.path])
        expect(result.current.draft).toEqual({ kind: 'file', parentDir: dir.path })
    })

    /** A directory that is neither the root nor a visible row has nowhere to draw the inline row, so no draft is opened at all rather than one that commits somewhere else. */
    test('트리에도 없고 루트도 아닌 명시 대상은 초안을 열지 않는다', async () => {
        const { result } = renderCrud({ rows: [], selectedRow: null })

        await act(async () => void (await result.current.startDraft('file', '/project/gone')))

        expect(result.current.draft).toBeNull()
    })
})
