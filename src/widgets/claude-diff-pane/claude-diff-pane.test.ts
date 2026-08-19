import { describe, expect, mock, test } from 'bun:test'
import type { PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'

/**
 * `claude-diff-pane.tsx` imports `@shared/lib/monaco/setup`, which pulls in real monaco-editor
 * worker bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve — `bun test`
 * cannot load them at all. Stubbing `@shared/lib/monaco/setup`, then reaching the module under test
 * through a *dynamic* `import()` (not a static import) is the same workaround
 * `ide-sync-provider.test.ts` documents. This project has no component-rendering test harness
 * (no `@testing-library/react`/DOM environment configured for `bun:test`), so this is a load-only
 * smoke test — the tab-close-vs-tab-switch distinction in the unmount effect (see that effect's own
 * doc comment) is verified by the pure `takePendingClaudeDiffIfUnresolved` behavior covered in
 * `claude-diff-registry.test.ts`, plus manual/QA verification of the actual close flow.
 */
const FAKE_MONACO = {
    editor: {
        createDiffEditor: () => ({ setModel: () => {}, dispose: () => {} }),
        createModel: () => ({ getValue: () => '', setValue: () => {}, dispose: () => {} }),
        setModelLanguage: () => {},
    },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

describe('ClaudeDiffPane 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', async () => {
        const imported = await import('@widgets/claude-diff-pane/claude-diff-pane')
        expect(typeof imported.ClaudeDiffPane).toBe('function')
    })
})

const buildTab = (id: string): Tab => ({ id, kind: { kind: 'file', path: `/${id}.ts` }, title: id })

const buildLayout = (overrides: Partial<ProjectLayout> = {}): ProjectLayout => ({
    version: 2,
    root: { node: 'leaf', id: 'main-leaf', tabs: [], active: null },
    focusedPane: 'main-leaf',
    ...overrides,
})

describe('isTabStillOpenInLayout', () => {
    test('layout 이 없으면(캐시 미스) 닫힌 것으로 취급한다', async () => {
        const { isTabStillOpenInLayout } = await import('@widgets/claude-diff-pane/claude-diff-pane')
        expect(isTabStillOpenInLayout(undefined, 'tab-1')).toBe(false)
    })

    test('메인 창 트리에 탭이 있으면 열려 있는 것으로 판단한다', async () => {
        const { isTabStillOpenInLayout } = await import('@widgets/claude-diff-pane/claude-diff-pane')
        const layout = buildLayout({ root: { node: 'leaf', id: 'main-leaf', tabs: [buildTab('tab-1')], active: 'tab-1' } })
        expect(isTabStillOpenInLayout(layout, 'tab-1')).toBe(true)
    })

    test('탭이 메인/보조 창 어디에도 없으면 닫힌 것으로 판단한다', async () => {
        const { isTabStillOpenInLayout } = await import('@widgets/claude-diff-pane/claude-diff-pane')
        const layout = buildLayout()
        expect(isTabStillOpenInLayout(layout, 'tab-1')).toBe(false)
    })

    test('탭이 "Move to Window" 로 보조 창으로 이동해 layout.root 에는 없어도 열려 있는 것으로 판단한다', async () => {
        const { isTabStillOpenInLayout } = await import('@widgets/claude-diff-pane/claude-diff-pane')
        const auxRoot: PaneNode = { node: 'leaf', id: 'aux-leaf', tabs: [buildTab('tab-1')], active: 'tab-1' }
        const layout = buildLayout({ auxiliaryWindows: [{ slot: 1, root: auxRoot, focusedPane: 'aux-leaf' }] })

        expect(isTabStillOpenInLayout(layout, 'tab-1')).toBe(true)
    })
})
