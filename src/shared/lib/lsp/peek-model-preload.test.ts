import { afterEach, describe, expect, test } from 'bun:test'
import type { FileSizeTier } from '@shared/api/bindings'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import {
    cancelPeekModelDispose,
    isPeekPreloadedModel,
    PEEK_MODEL_PRELOAD_LIMIT,
    type PeekModelPreloadFile,
    preloadPeekModel,
    preloadPeekModels,
    resetPeekModelPreloadStateForTests,
    selectPeekPreloadPaths,
} from '@shared/lib/lsp/peek-model-preload'

/**
 * `preloadPeekModel`'s dispose timer defaults to a real 60s TTL, which never fires within a test
 * run — left unreset, a path preloaded by one test here would leak into every other test *file*
 * that runs afterward in the same `bun test` process (bun does not isolate modules per file).
 */
afterEach(() => {
    resetPeekModelPreloadStateForTests()
})

const uriCache = new Map<string, { scheme: string; fsPath: string; toString: () => string }>()

const createFakeUri = (raw: string) => {
    const schemeMatch = /^([a-zA-Z][a-zA-Z\d+.-]*):\/\/(.*)$/.exec(raw)
    const scheme = schemeMatch ? schemeMatch[1] : 'file'
    const fsPath = schemeMatch ? schemeMatch[2] : raw
    const canonical = `${scheme}://${fsPath}`
    const cached = uriCache.get(canonical)
    if (cached) return cached
    const uri = { scheme, fsPath, toString: () => canonical }
    uriCache.set(canonical, uri)
    return uri
}

type FakeModel = {
    content: string
    languageId: string
    uri: ReturnType<typeof createFakeUri>
    disposed: boolean
    dispose: () => void
    isDisposed: () => boolean
}
type FakeEditor = { getModel: () => FakeModel | null }

const createFakeMonaco = () => {
    const models = new Map<string, FakeModel>()
    const editors: FakeEditor[] = []
    let createModelCallCount = 0

    const fakeMonaco = {
        Uri: { parse: createFakeUri, file: (path: string) => createFakeUri(`file://${path}`) },
        editor: {
            getModel: (uri: { toString: () => string }) => models.get(uri.toString()) ?? null,
            createModel: (content: string, languageId: string, uri: ReturnType<typeof createFakeUri>) => {
                createModelCallCount += 1
                const model: FakeModel = {
                    content,
                    languageId,
                    uri,
                    disposed: false,
                    dispose: () => (model.disposed = true),
                    isDisposed: () => model.disposed,
                }
                models.set(uri.toString(), model)
                return model
            },
            getEditors: () => editors,
        },
    }

    return {
        monaco: fakeMonaco as unknown as Monaco,
        models,
        editors,
        hasModelFor: (path: string) => models.has(`file://${path}`),
        getModelFor: (path: string) => models.get(`file://${path}`) ?? null,
        getCreateModelCallCount: () => createModelCallCount,
    }
}

const normalFile = (content = 'content'): PeekModelPreloadFile => ({ content, languageId: 'typescript', tier: 'normal' as FileSizeTier })

const TEST_DISPOSE_DELAY_MS = 20
const waitPastDisposeDelay = () => new Promise((resolve) => setTimeout(resolve, TEST_DISPOSE_DELAY_MS + 30))

describe('selectPeekPreloadPaths', () => {
    test('중복을 제거하고 이미 모델이 있는 경로는 제외한다', () => {
        const result = selectPeekPreloadPaths(['/a.ts', '/b.ts', '/a.ts'], (path) => path === '/b.ts')
        expect(result).toEqual(['/a.ts'])
    })

    test('limit 을 초과하는 경로는 잘라낸다', () => {
        const paths = ['/a.ts', '/b.ts', '/c.ts', '/d.ts']
        const result = selectPeekPreloadPaths(paths, () => false, 2)
        expect(result).toEqual(['/a.ts', '/b.ts'])
    })

    test('기본 limit 은 PEEK_MODEL_PRELOAD_LIMIT 이다', () => {
        const paths = Array.from({ length: PEEK_MODEL_PRELOAD_LIMIT + 5 }, (_, index) => `/f${index}.ts`)
        const result = selectPeekPreloadPaths(paths, () => false)
        expect(result).toHaveLength(PEEK_MODEL_PRELOAD_LIMIT)
    })
})

describe('preloadPeekModel', () => {
    test('모델이 없는 파일은 읽어서 모델을 생성한다', async () => {
        const { monaco, hasModelFor } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/target.ts', { readFile: async () => normalFile() })

        expect(hasModelFor('/workspace/target.ts')).toBe(true)
    })

    test('이미 모델이 있으면 읽기를 시도하지 않는다', async () => {
        const { monaco } = createFakeMonaco()
        monaco.editor.createModel('existing', 'typescript', monaco.Uri.file('/workspace/target.ts'))
        let readCount = 0

        await preloadPeekModel(monaco, '/workspace/target.ts', {
            readFile: async () => {
                readCount += 1
                return normalFile()
            },
        })

        expect(readCount).toBe(0)
    })

    test('large/readOnly/refused tier 파일은 모델을 생성하지 않는다 (대용량 파일 가드)', async () => {
        const { monaco, hasModelFor } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/large.ts', { readFile: async () => ({ content: 'x', languageId: 'typescript', tier: 'large' }) })
        await preloadPeekModel(monaco, '/workspace/ro.ts', {
            readFile: async () => ({ content: 'x', languageId: 'typescript', tier: 'readOnly' }),
        })
        await preloadPeekModel(monaco, '/workspace/bin', { readFile: async () => ({ content: '', languageId: 'plaintext', tier: 'refused' }) })

        expect(hasModelFor('/workspace/large.ts')).toBe(false)
        expect(hasModelFor('/workspace/ro.ts')).toBe(false)
        expect(hasModelFor('/workspace/bin')).toBe(false)
    })

    test('읽기가 실패하면 예외를 전파한다 (preloadPeekModels 가 이를 스윕한다)', async () => {
        const { monaco } = createFakeMonaco()

        await expect(
            preloadPeekModel(monaco, '/workspace/missing.ts', {
                readFile: async () => {
                    throw new Error('file not found')
                },
            }),
        ).rejects.toThrow('file not found')
    })
})

describe('preloadPeekModel — 모델 수명 (dispose TTL)', () => {
    test('아무 에디터도 표시하지 않으면 지연 시간 후 자동으로 dispose 된다', async () => {
        const { monaco, getModelFor } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/target.ts', { readFile: async () => normalFile(), disposeDelayMs: TEST_DISPOSE_DELAY_MS })
        await waitPastDisposeDelay()

        expect(getModelFor('/workspace/target.ts')?.isDisposed()).toBe(true)
    })

    test('에디터가 여전히 모델을 표시 중이면 dispose 하지 않는다 (peek 미리보기 보호)', async () => {
        const { monaco, editors, getModelFor } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/target.ts', { readFile: async () => normalFile(), disposeDelayMs: TEST_DISPOSE_DELAY_MS })
        const model = getModelFor('/workspace/target.ts')
        editors.push({ getModel: () => model })
        await waitPastDisposeDelay()

        expect(model?.isDisposed()).toBe(false)
    })
})

describe('isPeekPreloadedModel / cancelPeekModelDispose — 소유권 이관', () => {
    test('프리로드 직후에는 peek 전용 모델로 표시된다', async () => {
        const { monaco } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/owned.ts', { readFile: async () => normalFile() })

        expect(isPeekPreloadedModel('/workspace/owned.ts')).toBe(true)
    })

    test('cancelPeekModelDispose 호출 후에는 peek 전용 모델로 표시되지 않고, TTL 이 지나도 dispose 되지 않는다 (탭 입양 시나리오)', async () => {
        const { monaco, getModelFor } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/adopted.ts', { readFile: async () => normalFile(), disposeDelayMs: TEST_DISPOSE_DELAY_MS })
        cancelPeekModelDispose('/workspace/adopted.ts')
        await waitPastDisposeDelay()

        expect(isPeekPreloadedModel('/workspace/adopted.ts')).toBe(false)
        expect(getModelFor('/workspace/adopted.ts')?.isDisposed()).toBe(false)
    })

    test('TTL 로 dispose 되면 peek 전용 모델 표시도 함께 해제된다', async () => {
        const { monaco } = createFakeMonaco()

        await preloadPeekModel(monaco, '/workspace/expired.ts', { readFile: async () => normalFile(), disposeDelayMs: TEST_DISPOSE_DELAY_MS })
        await waitPastDisposeDelay()

        expect(isPeekPreloadedModel('/workspace/expired.ts')).toBe(false)
    })
})

describe('preloadPeekModels', () => {
    test('대상 파일들을 모두 프리로드한다', async () => {
        const { monaco, hasModelFor } = createFakeMonaco()

        await preloadPeekModels(monaco, ['/a.ts', '/b.ts'], { readFile: async (path) => normalFile(`content of ${path}`) })

        expect(hasModelFor('/a.ts')).toBe(true)
        expect(hasModelFor('/b.ts')).toBe(true)
    })

    test('중복 경로는 한 번만 읽는다', async () => {
        const { monaco } = createFakeMonaco()
        let readCount = 0

        await preloadPeekModels(monaco, ['/a.ts', '/a.ts', '/a.ts'], {
            readFile: async () => {
                readCount += 1
                return normalFile()
            },
        })

        expect(readCount).toBe(1)
    })

    test('PEEK_MODEL_PRELOAD_LIMIT 을 초과하는 요청은 초과분을 건너뛴다', async () => {
        const { monaco, getCreateModelCallCount } = createFakeMonaco()
        const paths = Array.from({ length: PEEK_MODEL_PRELOAD_LIMIT + 10 }, (_, index) => `/f${index}.ts`)

        await preloadPeekModels(monaco, paths, { readFile: async () => normalFile() })

        expect(getCreateModelCallCount()).toBe(PEEK_MODEL_PRELOAD_LIMIT)
    })

    test('한 파일의 읽기 실패가 나머지 파일 프리로드를 막지 않는다', async () => {
        const { monaco, hasModelFor } = createFakeMonaco()

        await preloadPeekModels(monaco, ['/broken.ts', '/ok.ts'], {
            readFile: async (path) => {
                if (path === '/broken.ts') throw new Error('cannot read')
                return normalFile()
            },
        })

        expect(hasModelFor('/broken.ts')).toBe(false)
        expect(hasModelFor('/ok.ts')).toBe(true)
    })

    test('동시에 같은 경로를 여러 번 프리로드해도 모델을 한 번만 만든다 (경합 방지)', async () => {
        const { monaco, getCreateModelCallCount } = createFakeMonaco()

        await Promise.all([
            preloadPeekModel(monaco, '/race.ts', { readFile: async () => normalFile('first') }),
            preloadPeekModel(monaco, '/race.ts', { readFile: async () => normalFile('second') }),
        ])

        expect(getCreateModelCallCount()).toBe(1)
    })
})
