import { describe, expect, test } from 'bun:test'
import type { ProjectLayout, Tab } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { planOpenToTheSide } from '@widgets/explorer/open-to-the-side-plan'

const PROJECT_ID = 'project-1'
const AUX_WINDOW_SLOT = 1

const buildRow = (path: string, kind: FileTreeRow['kind']): FileTreeRow => ({
    id: path,
    path,
    name: path.slice(path.lastIndexOf('/') + 1),
    depth: 0,
    kind,
    expanded: false,
    gitStatus: null,
})

const buildTab = (path: string): Tab => ({ id: path, kind: { kind: 'file', path }, title: path })

const buildLayout = (tabs: Tab[], auxiliaryWindows?: ProjectLayout['auxiliaryWindows']): ProjectLayout => ({
    version: 2,
    root: { node: 'leaf', id: 'main-leaf', tabs, active: tabs[0]?.id ?? null },
    focusedPane: 'main-leaf',
    revision: 1,
    auxiliaryWindows,
})

/** The harness pins `location` to the main window (`docs/memory/test-conventions.md` §4), so the auxiliary case sets and restores it around the assertion. */
const withAuxiliaryWindowLocation = (run: () => void) => {
    window.history.replaceState({}, '', `/?projectId=${PROJECT_ID}&windowSlot=${AUX_WINDOW_SLOT}`)
    try {
        run()
    } finally {
        window.history.replaceState({}, '', '/')
    }
}

const file = buildRow('/project/src/app.tsx', 'file')

describe('planOpenToTheSide', () => {
    /**
     * The whole point of the single `layout_open_tab_in_split`: the request exists even when the
     * focused group holds no tab at all, which is where the previous open-then-split pair ended with
     * no split (the emptied source leaf was normalized away).
     */
    test('탭이 없는 빈 그룹에서도 분할 요청을 만든다', () => {
        expect(planOpenToTheSide(PROJECT_ID, file, buildLayout([]))).toEqual({
            projectId: PROJECT_ID,
            targetPane: 'main-leaf',
            edge: 'right',
            kind: { kind: 'file', path: file.path },
            title: file.name,
            preview: false,
        })
    })

    /** Already open in the focused group is the other collapse case — and `open_tab_in_split` has no kind dedupe, so this really does open a second tab beside the first. */
    test('같은 파일 탭 하나만 열려 있어도 분할 요청은 그대로 만들어진다', () => {
        expect(planOpenToTheSide(PROJECT_ID, file, buildLayout([buildTab(file.path)]))?.targetPane).toBe('main-leaf')
    })

    test('디렉터리 행은 옆에 열 것이 없으므로 요청을 만들지 않는다', () => {
        expect(planOpenToTheSide(PROJECT_ID, buildRow('/project/src', 'directory'), buildLayout([]))).toBeNull()
    })

    test('레이아웃이 아직 없으면 분할할 페인도 없다', () => {
        expect(planOpenToTheSide(PROJECT_ID, file, undefined)).toBeNull()
    })

    test('보조 창에서는 그 창의 포커스 페인을 분할한다 (메인 트리의 focusedPane 이 아니다)', () => {
        withAuxiliaryWindowLocation(() => {
            const layout = buildLayout(
                [buildTab('/project/src/main.tsx')],
                [{ slot: AUX_WINDOW_SLOT, root: { node: 'leaf', id: 'aux-leaf', tabs: [], active: null }, focusedPane: 'aux-leaf' }],
            )

            expect(planOpenToTheSide(PROJECT_ID, file, layout)?.targetPane).toBe('aux-leaf')
        })
    })

    test('보조 창 슬롯이 레이아웃에 없으면 요청을 만들지 않는다 (메인 창으로 새지 않는다)', () => {
        withAuxiliaryWindowLocation(() => {
            expect(planOpenToTheSide(PROJECT_ID, file, buildLayout([buildTab('/project/src/main.tsx')]))).toBeNull()
        })
    })
})
