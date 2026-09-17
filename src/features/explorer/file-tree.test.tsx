import { describe, expect, test } from 'bun:test'
import type { FileTreeContextMenuHandlers, FileTreeDraft } from '@features/explorer/file-tree'
import { FileTree } from '@features/explorer/file-tree'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

const ROW_HEIGHT_PX = 22

const ROWS: FileTreeRow[] = [
    { id: '/p/src', path: '/p/src', name: 'src', depth: 0, kind: 'directory', expanded: false, gitStatus: null },
    { id: '/p/a.ts', path: '/p/a.ts', name: 'a.ts', depth: 0, kind: 'file', expanded: false, gitStatus: null },
]

type Call = { name: string; path: string | null }

const noop = () => {}

const renderTree = ({ draft = null }: { draft?: FileTreeDraft | null } = {}) => {
    const calls: Call[] = []
    const record =
        (name: string) =>
        (row: FileTreeRow | null = null) =>
            void calls.push({ name, path: row?.path ?? null })
    const selectionChanges: (string | null)[] = []

    const contextMenuHandlers: FileTreeContextMenuHandlers = {
        onOpenToTheSide: record('openToTheSide'),
        onOpenWithEditor: record('openWithEditor'),
        onOpenWithPreview: record('openWithPreview'),
        onOpenInBrowser: record('openInBrowser'),
        onRevealInFinder: record('revealInFinder'),
        onOpenInTerminal: record('openInTerminal'),
        onFindInFolder: record('findInFolder'),
        onSelectForCompare: record('selectForCompare'),
        onCompareWithSelected: record('compareWithSelected'),
        canCompareWithSelected: false,
        onFileHistory: record('fileHistory'),
        onCut: record('cut'),
        onCopy: record('copy'),
        onPaste: record('paste'),
        onCopyPath: record('copyPath'),
        onCopyRelativePath: record('copyRelativePath'),
        onStartRename: record('startRename'),
        onRequestDelete: record('requestDelete'),
    }

    const rendered = renderWithProviders(
        <FileTree
            rows={ROWS}
            draft={draft}
            draftError={null}
            renameTarget={null}
            renameError={null}
            selectPathRequest={null}
            canPaste={false}
            contextMenuHandlers={contextMenuHandlers}
            onToggleExpand={record('toggleExpand')}
            onOpenPreview={record('openPreview')}
            onOpenPinned={record('openPinned')}
            onSelectionChange={(id) => void selectionChanges.push(id)}
            onDraftCommit={noop}
            onDraftCancel={noop}
            onRenameCommit={noop}
            onRenameCancel={noop}
            onSelectPathRequestHandled={noop}
            onNewFile={record('newFile')}
            onNewFolder={record('newFolder')}
            onNewFileAtRoot={record('newFileAtRoot')}
        />,
    )

    return { ...rendered, calls, selectionChanges, tree: screen.getByRole('tree') }
}

const selectFirstRow = (tree: HTMLElement) => fireEvent.keyDown(tree, { key: 'ArrowDown' })

describe('FileTree 단축키', () => {
    const cases: { name: string; key: string; init?: Record<string, boolean>; expected: string }[] = [
        { name: 'Enter 는 이름 바꾸기', key: 'Enter', expected: 'startRename' },
        { name: 'F2 도 이름 바꾸기', key: 'F2', expected: 'startRename' },
        { name: '⌘↓ 는 고정 탭 열기', key: 'ArrowDown', init: { metaKey: true }, expected: 'openPinned' },
        { name: 'Space 는 미리보기', key: ' ', expected: 'openPreview' },
        { name: '⌘⌫ 는 삭제 요청', key: 'Backspace', init: { metaKey: true }, expected: 'requestDelete' },
        { name: '⌘X 는 잘라내기', key: 'x', init: { metaKey: true }, expected: 'cut' },
        { name: '⌘C 는 복사', key: 'c', init: { metaKey: true }, expected: 'copy' },
        { name: '⌘V 는 붙여넣기', key: 'v', init: { metaKey: true }, expected: 'paste' },
        { name: '⌥⌘R 은 Finder 표시', key: 'r', init: { metaKey: true, altKey: true }, expected: 'revealInFinder' },
        { name: '⌥⌘C 는 경로 복사', key: 'c', init: { metaKey: true, altKey: true }, expected: 'copyPath' },
        { name: '⇧⌥⌘C 는 상대 경로 복사', key: 'c', init: { metaKey: true, altKey: true, shiftKey: true }, expected: 'copyRelativePath' },
    ]

    for (const { name, key, init, expected } of cases) {
        test(`${name} 를 선택 행에 대해 호출한다`, () => {
            const { tree, calls } = renderTree()
            selectFirstRow(tree)

            fireEvent.keyDown(tree, { key, ...init })

            expect(calls.filter((call) => call.name === expected)).toEqual([{ name: expected, path: ROWS[0].path }])
        })
    }

    test('⌘N 은 새 파일, ⌘⇧N 은 새 폴더 초안을 연다', () => {
        const { tree, calls } = renderTree()
        selectFirstRow(tree)

        fireEvent.keyDown(tree, { key: 'n', metaKey: true })
        fireEvent.keyDown(tree, { key: 'n', metaKey: true, shiftKey: true })

        expect(calls.map((call) => call.name)).toEqual(['newFile', 'newFolder'])
    })

    test('선택이 없어도 붙여넣기는 대상 없음(null)으로 전달된다', () => {
        const { tree, calls } = renderTree()

        fireEvent.keyDown(tree, { key: 'v', metaKey: true })

        expect(calls).toEqual([{ name: 'paste', path: null }])
    })

    test('선택이 없으면 행이 필요한 단축키는 아무것도 호출하지 않는다', () => {
        const { tree, calls } = renderTree()

        fireEvent.keyDown(tree, { key: 'Enter' })
        fireEvent.keyDown(tree, { key: ' ' })

        expect(calls).toEqual([])
    })

    /**
     * The tree's rows start at the project root's *children* (Rust `push_page_rows`), so the only
     * depth-0 synthetic row that can ever sit above them is the inline draft — and while it exists
     * the whole handler is disabled, which is what keeps Enter from renaming a nameless row.
     */
    test('초안 편집 중에는 어떤 단축키도 동작하지 않는다', () => {
        const { tree, calls } = renderTree({ draft: { kind: 'file', parentDir: '/p' } })

        fireEvent.keyDown(tree, { key: 'ArrowDown' })
        fireEvent.keyDown(tree, { key: 'Enter' })
        fireEvent.keyDown(tree, { key: 'Backspace', metaKey: true })

        expect(calls).toEqual([])
    })

    test('IME 조합 중 keydown 은 무시한다', () => {
        const { tree, calls } = renderTree()
        selectFirstRow(tree)

        fireEvent.keyDown(tree, { key: 'Enter', keyCode: 229 })

        expect(calls).toEqual([])
    })

    test('수식키 없는 방향키는 그대로 트리 탐색으로 남는다', () => {
        const { tree, calls } = renderTree()
        selectFirstRow(tree)

        fireEvent.keyDown(tree, { key: 'ArrowRight' })

        expect(calls).toEqual([{ name: 'toggleExpand', path: ROWS[0].path }])
    })
})

describe('FileTree 빈 공간 더블클릭', () => {
    test('행이 없는 아래쪽을 더블클릭하면 선택을 지우고 루트 새 파일 초안을 연다', () => {
        const { tree, calls, selectionChanges } = renderTree()
        selectFirstRow(tree)

        fireEvent.doubleClick(tree, { clientY: ROWS.length * ROW_HEIGHT_PX + 1 })

        expect(selectionChanges).toEqual([ROWS[0].id, null])
        expect(calls.map((call) => call.name)).toEqual(['newFileAtRoot'])
    })

    /**
     * The row's own `onDoubleClick` (unchanged: pinned open) is not reachable here — the virtualizer
     * measures a zero-height viewport under happy-dom and mounts no rows — so what is pinned instead
     * is the half this change owns: a double-click that lands *on* a row is left to that row handler
     * and never falls through to the root draft.
     */
    test('행 위 더블클릭은 행 핸들러 몫이라 루트 초안을 열지 않는다', () => {
        const { tree, calls } = renderTree()

        fireEvent.doubleClick(tree, { clientY: 1 })

        expect(calls).toEqual([])
    })

    test('초안 편집 중에는 빈 공간 더블클릭이 동작하지 않는다', () => {
        const { tree, calls } = renderTree({ draft: { kind: 'file', parentDir: '/p' } })

        fireEvent.doubleClick(tree, { clientY: ROWS.length * ROW_HEIGHT_PX + 100 })

        expect(calls).toEqual([])
    })
})
