import { beforeEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import type { MirrorEntry, OpenedFile, PaneNode, ProjectLayout, Tab, TabPathChange, TabPathChangeResult } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `tab-path-change.ts` reaches `@entities/editor/model-registry` (monaco worker bundles `bun test`
 * cannot load) and the two `.ipc` modules (Tauri command bindings) at import time, so both are
 * stubbed before the module is pulled in through a *dynamic* `import()` — the workaround
 * `reveal-registry.test.ts`/`git.query.test.ts` document. The behavior under test is driven through
 * the injected `deps` seam instead of those stubs, which only need to exist for the module's own
 * default-deps object to be constructible.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))
mock.module('@entities/layout/layout.ipc', () => ({ applyTabPathChange: () => Promise.resolve(null) }))
mock.module('@entities/file/file.ipc', () => ({
    openFile: () => Promise.resolve(null),
    mirrorDirty: () => Promise.resolve(null),
    clearMirror: () => Promise.resolve(null),
}))
mock.module('@entities/agent/agent.ipc', () => ({ releaseWaitMarker: () => Promise.resolve(null) }))

const importTabPathChange = () => import('@entities/layout/tab-path-change')

const PROJECT_ID = 'project-1'

const buildFileTab = (id: string, path: string): Tab => ({ id, kind: { kind: 'file', path }, title: path, dirty: false })

const buildLeaf = (tabs: Tab[]): PaneNode => ({ node: 'leaf', id: 'leaf', tabs, active: tabs[0]?.id ?? null })

const buildLayout = (tabs: Tab[], revision = 1): ProjectLayout => ({
    version: 2,
    root: buildLeaf(tabs),
    focusedPane: 'leaf',
    revision,
    closedTabs: [],
    auxiliaryWindows: [],
})

const buildOpenedFile = (path: string, content: string, languageId: string): OpenedFile => ({
    path,
    content,
    languageId,
    byteSize: content.length,
    lineCount: 1,
    tier: 'normal',
    readOnly: false,
    encodingLossy: false,
    modifiedMs: 1_700_000_000_000,
    editorConfig: { indentStyle: null, indentSize: null, tabWidth: null, insertFinalNewline: null, trimTrailingWhitespace: null },
})

type Recorder = {
    changes: TabPathChange[]
    /** Interleaved `mirrorDirty`/`clearMirror` order, which the two lists below cannot express on their own. */
    mirrorOps: string[]
    mirrorWrites: { path: string; content: string }[]
    clearedMirrors: string[]
    retargeted: { from: string; to: string }[]
    languages: { path: string; languageId: string }[]
    disposed: string[]
    overrides: { path: string; override: 'editor' | null }[]
    releasedMarkers: string[]
}

const createDeps = (
    recorder: Recorder,
    options: { result: TabPathChangeResult; modelContents?: Record<string, string>; openedFiles?: Record<string, OpenedFile> },
) => ({
    applyTabPathChange: (input: { projectId: string; change: TabPathChange }) => {
        recorder.changes.push(input.change)
        return Promise.resolve(options.result)
    },
    openFile: (path: string) => {
        const opened = options.openedFiles?.[path]
        return opened ? Promise.resolve(opened) : Promise.reject(new Error(`no file: ${path}`))
    },
    mirrorDirty: (input: { path: string; content: string }) => {
        recorder.mirrorOps.push(`write:${input.path}`)
        recorder.mirrorWrites.push({ path: input.path, content: input.content })
        return Promise.resolve(1_700_000_000_000)
    },
    clearMirror: (input: { path: string }) => {
        recorder.mirrorOps.push(`clear:${input.path}`)
        recorder.clearedMirrors.push(input.path)
        return Promise.resolve(null)
    },
    readModelContent: (path: string) => options.modelContents?.[path] ?? null,
    retargetModel: (from: string, to: string) => {
        recorder.retargeted.push({ from, to })
    },
    applyModelLanguage: (path: string, languageId: string) => {
        recorder.languages.push({ path, languageId })
    },
    disposeModel: (path: string) => {
        recorder.disposed.push(path)
    },
    getOpenWithOverride: () => null,
    setOpenWithOverride: (path: string, override: 'editor' | null) => {
        recorder.overrides.push({ path, override })
    },
    takeWaitMarkers: (path: string) => (path === '/repo/marked.ts' ? ['marker-1'] : []),
    releaseWaitMarker: (marker: string) => {
        recorder.releasedMarkers.push(marker)
        return Promise.resolve(null)
    },
})

let recorder: Recorder

beforeEach(() => {
    recorder = {
        changes: [],
        mirrorOps: [],
        mirrorWrites: [],
        clearedMirrors: [],
        retargeted: [],
        languages: [],
        disposed: [],
        overrides: [],
        releasedMarkers: [],
    }
})

describe('followRenamedPathInTabs', () => {
    test('개명된 경로의 모델·FILE 캐시를 옮기고 새 languageId 를 적용한다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.FILE.CONTENT('/repo/notes.txt'), buildOpenedFile('/repo/notes.txt', 'body', 'plaintext'))
        const layout = buildLayout([buildFileTab('tab-1', '/repo/notes.ts')], 5)
        const deps = createDeps(recorder, {
            result: { layout, moved: [{ from: '/repo/notes.txt', to: '/repo/notes.ts', dirty: false }], closedPaths: [] },
            openedFiles: { '/repo/notes.ts': buildOpenedFile('/repo/notes.ts', 'body', 'typescript') },
        })

        const result = await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/notes.txt', to: '/repo/notes.ts' }, deps)

        expect(recorder.changes).toEqual([{ kind: 'renamed', from: '/repo/notes.txt', to: '/repo/notes.ts' }])
        expect(recorder.retargeted).toEqual([{ from: '/repo/notes.txt', to: '/repo/notes.ts' }])
        expect(queryClient.getQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT('/repo/notes.ts'))?.languageId).toBe('typescript')
        expect(recorder.languages).toEqual([{ path: '/repo/notes.ts', languageId: 'typescript' }])
        expect(recorder.mirrorWrites).toEqual([])
        expect(result.layout.revision).toBe(5)
    })

    test('미저장 편집이 있으면 라이브 모델 내용을 새 경로의 미러로 옮기고 옛 미러를 지운다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        const staleMirror: MirrorEntry = {
            path: '/repo/a.ts',
            content: 'debounced older draft',
            savedAtMs: 1,
            diskModifiedMs: 1,
            conflict: false,
        }
        queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID), [staleMirror])
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/b.ts')]),
                moved: [{ from: '/repo/a.ts', to: '/repo/b.ts', dirty: true }],
                closedPaths: [],
            },
            modelContents: { '/repo/a.ts': 'freshest draft' },
            openedFiles: { '/repo/b.ts': buildOpenedFile('/repo/b.ts', 'disk', 'typescript') },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/a.ts', to: '/repo/b.ts' }, deps)

        expect(recorder.mirrorWrites).toEqual([{ path: '/repo/b.ts', content: 'freshest draft' }])
        expect(recorder.clearedMirrors).toEqual(['/repo/a.ts'])
        expect(queryClient.getQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID))).toEqual([
            { path: '/repo/b.ts', content: 'freshest draft', savedAtMs: expect.any(Number), diskModifiedMs: 1_700_000_000_000, conflict: false },
        ])
    })

    test('옛 미러를 먼저 지운 뒤에 새 경로 미러를 쓴다 (대소문자만 다른 개명이 방금 쓴 미러를 지우지 않도록)', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID), [
            { path: '/repo/readme.md', content: 'older draft', savedAtMs: 1, diskModifiedMs: 1, conflict: false },
        ])
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/README.md')]),
                moved: [{ from: '/repo/readme.md', to: '/repo/README.md', dirty: true }],
                closedPaths: [],
            },
            modelContents: { '/repo/readme.md': 'live draft' },
            openedFiles: { '/repo/README.md': buildOpenedFile('/repo/README.md', 'disk', 'markdown') },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/readme.md', to: '/repo/README.md' }, deps)

        expect(recorder.mirrorOps).toEqual(['clear:/repo/readme.md', 'write:/repo/README.md'])
        expect(queryClient.getQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID))).toEqual([
            { path: '/repo/README.md', content: 'live draft', savedAtMs: expect.any(Number), diskModifiedMs: 1_700_000_000_000, conflict: false },
        ])
    })

    test('모델이 없는 탭은 미러 내용으로 대신 옮긴다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        queryClient.setQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID), [
            { path: '/repo/a.ts', content: 'mirror draft', savedAtMs: 1, diskModifiedMs: 1, conflict: false },
        ])
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/b.ts')]),
                moved: [{ from: '/repo/a.ts', to: '/repo/b.ts', dirty: true }],
                closedPaths: [],
            },
            openedFiles: { '/repo/b.ts': buildOpenedFile('/repo/b.ts', 'disk', 'typescript') },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/a.ts', to: '/repo/b.ts' }, deps)

        expect(recorder.mirrorWrites).toEqual([{ path: '/repo/b.ts', content: 'mirror draft' }])
    })

    test('미러 목록을 아직 조회한 적 없는 프로젝트의 캐시는 부분 목록으로 심지 않는다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/b.ts')]),
                moved: [{ from: '/repo/a.ts', to: '/repo/b.ts', dirty: true }],
                closedPaths: [],
            },
            modelContents: { '/repo/a.ts': 'draft' },
            openedFiles: { '/repo/b.ts': buildOpenedFile('/repo/b.ts', 'disk', 'typescript') },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/a.ts', to: '/repo/b.ts' }, deps)

        expect(recorder.mirrorWrites).toEqual([{ path: '/repo/b.ts', content: 'draft' }])
        expect(queryClient.getQueryData<MirrorEntry[]>(QUERY_KEY.FILE.MIRRORS(PROJECT_ID))).toBeUndefined()
    })

    test('폴더 개명으로 여러 경로가 움직이면 각 경로를 개별로 이관한다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/lib/a.ts'), buildFileTab('tab-2', '/repo/lib/nested/b.ts')]),
                moved: [
                    { from: '/repo/src/a.ts', to: '/repo/lib/a.ts', dirty: false },
                    { from: '/repo/src/nested/b.ts', to: '/repo/lib/nested/b.ts', dirty: false },
                ],
                closedPaths: [],
            },
            openedFiles: {
                '/repo/lib/a.ts': buildOpenedFile('/repo/lib/a.ts', 'a', 'typescript'),
                '/repo/lib/nested/b.ts': buildOpenedFile('/repo/lib/nested/b.ts', 'b', 'typescript'),
            },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/src', to: '/repo/lib' }, deps)

        expect(recorder.retargeted).toEqual([
            { from: '/repo/src/a.ts', to: '/repo/lib/a.ts' },
            { from: '/repo/src/nested/b.ts', to: '/repo/lib/nested/b.ts' },
        ])
    })

    test('새 경로를 다시 읽지 못해도 실패하지 않고 FILE 캐시 무효화로 넘어간다', async () => {
        const { followRenamedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        const deps = createDeps(recorder, {
            result: {
                layout: buildLayout([buildFileTab('tab-1', '/repo/b.ts')]),
                moved: [{ from: '/repo/a.ts', to: '/repo/b.ts', dirty: false }],
                closedPaths: [],
            },
        })

        await followRenamedPathInTabs({ queryClient, projectId: PROJECT_ID, from: '/repo/a.ts', to: '/repo/b.ts' }, deps)

        expect(recorder.retargeted).toEqual([{ from: '/repo/a.ts', to: '/repo/b.ts' }])
        expect(recorder.languages).toEqual([])
    })
})

describe('followDeletedPathInTabs', () => {
    test('닫힌 경로마다 미러를 지우고 모델을 폐기한다', async () => {
        const { followDeletedPathInTabs } = await importTabPathChange()
        const queryClient = new QueryClient()
        const deps = createDeps(recorder, {
            result: { layout: buildLayout([]), moved: [], closedPaths: ['/repo/src/a.ts', '/repo/src/b.ts'] },
        })

        await followDeletedPathInTabs({ queryClient, projectId: PROJECT_ID, path: '/repo/src' }, deps)

        expect(recorder.changes).toEqual([{ kind: 'deleted', path: '/repo/src' }])
        expect(recorder.clearedMirrors).toEqual(['/repo/src/a.ts', '/repo/src/b.ts'])
        expect(recorder.disposed).toEqual(['/repo/src/a.ts', '/repo/src/b.ts'])
        expect(recorder.overrides).toEqual([
            { path: '/repo/src/a.ts', override: null },
            { path: '/repo/src/b.ts', override: null },
        ])
    })
})

describe('releaseClosedFileTabPath', () => {
    test('다른 페인·창에 같은 파일이 남아 있으면 모델을 폐기하지 않는다', async () => {
        const { releaseClosedFileTabPath } = await importTabPathChange()
        const queryClient = new QueryClient()
        const layout = buildLayout([buildFileTab('tab-2', '/repo/split.ts')])
        const deps = createDeps(recorder, { result: { layout, moved: [], closedPaths: [] } })

        releaseClosedFileTabPath({ queryClient, projectId: PROJECT_ID, path: '/repo/split.ts', layout }, deps)

        expect(recorder.clearedMirrors).toEqual(['/repo/split.ts'])
        expect(recorder.disposed).toEqual([])
        expect(recorder.overrides).toEqual([])
    })

    test('어디에도 열려 있지 않으면 모델을 폐기하고 대기 마커를 반납한다', async () => {
        const { releaseClosedFileTabPath } = await importTabPathChange()
        const queryClient = new QueryClient()
        const layout = buildLayout([])
        const deps = createDeps(recorder, { result: { layout, moved: [], closedPaths: [] } })

        releaseClosedFileTabPath({ queryClient, projectId: PROJECT_ID, path: '/repo/marked.ts', layout }, deps)

        expect(recorder.releasedMarkers).toEqual(['marker-1'])
        expect(recorder.disposed).toEqual(['/repo/marked.ts'])
    })
})
