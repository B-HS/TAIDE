import { beforeEach, describe, expect, mock, test } from 'bun:test'

/**
 * `model-registry.ts` imports `@shared/lib/monaco/setup`, which pulls in real monaco-editor worker
 * bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve — `bun test` cannot
 * load them at all. Stubbing that module and reaching the module under test through a *dynamic*
 * `import()` is the same workaround `reveal-registry.test.ts` documents.
 *
 * The fake models/editors below reproduce exactly the monaco behavior `retargetModel` depends on:
 * a uri is immutable and unique per model (`createModel` throws on a duplicate, as monaco's model
 * service does), and an editor holds whatever model was last `setModel`-ed on it.
 */
type FakeModel = {
    uri: { path: string; toString: () => string }
    value: string
    languageId: string
    disposed: boolean
    getValue: () => string
    getLanguageId: () => string
    isDisposed: () => boolean
    dispose: () => void
}

type FakeEditor = {
    model: FakeModel | null
    restoredViewStates: string[]
    getModel: () => FakeModel | null
    setModel: (model: FakeModel) => void
    saveViewState: () => string | null
    restoreViewState: (viewState: string) => void
}

let fakeModels: FakeModel[] = []
let fakeEditors: FakeEditor[] = []

const createFakeModel = (path: string, value: string, languageId: string): FakeModel => {
    const uriString = `file://${path}`
    if (fakeModels.some((model) => !model.disposed && model.uri.toString() === uriString)) {
        throw new Error(`duplicate model uri: ${uriString}`)
    }
    const model: FakeModel = {
        uri: { path, toString: () => uriString },
        value,
        languageId,
        disposed: false,
        getValue: () => model.value,
        getLanguageId: () => model.languageId,
        isDisposed: () => model.disposed,
        dispose: () => {
            model.disposed = true
        },
    }
    fakeModels.push(model)
    return model
}

const createFakeEditor = (model: FakeModel | null, viewState: string | null): FakeEditor => {
    const editor: FakeEditor = {
        model,
        restoredViewStates: [],
        getModel: () => editor.model,
        setModel: (next) => {
            editor.model = next
        },
        saveViewState: () => viewState,
        restoreViewState: (state) => editor.restoredViewStates.push(state),
    }
    fakeEditors.push(editor)
    return editor
}

const FAKE_MONACO = {
    Uri: {
        file: (path: string) => ({ path, toString: () => `file://${path}` }),
        parse: (value: string) => ({ path: value, toString: () => value }),
    },
    editor: {
        createModel: (value: string, languageId: string, uri: { path: string }) => createFakeModel(uri.path, value, languageId),
        getModel: (uri: { toString: () => string }) => fakeModels.find((model) => !model.disposed && model.uri.toString() === uri.toString()) ?? null,
        getEditors: () => fakeEditors,
        setModelLanguage: (model: FakeModel, languageId: string) => {
            model.languageId = languageId
        },
    },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

const importModelRegistry = () => import('@entities/editor/model-registry')

beforeEach(() => {
    fakeModels = []
    fakeEditors = []
})

describe('retargetModel', () => {
    test('개명된 경로로 버퍼·언어를 옮기고 옛 모델은 폐기한다', async () => {
        const { getOrCreateModel, getModel, retargetModel } = await importModelRegistry()
        const source = getOrCreateModel('/repo/old.ts', 'edited buffer', 'typescript')

        retargetModel('/repo/old.ts', '/repo/new.ts')

        expect(getModel('/repo/old.ts')).toBeUndefined()
        const moved = getModel('/repo/new.ts')
        expect(moved?.getValue()).toBe('edited buffer')
        expect(moved?.getLanguageId()).toBe('typescript')
        expect(moved?.uri.toString()).toBe('file:///repo/new.ts')
        expect(source.isDisposed()).toBe(true)
    })

    test('그 모델을 띄우고 있던 에디터는 폐기 전에 새 모델로 옮겨지고 뷰 상태가 복원된다', async () => {
        const { getOrCreateModel, retargetModel } = await importModelRegistry()
        const source = getOrCreateModel('/repo/attached-old.ts', 'body', 'typescript')
        const editor = createFakeEditor(source as unknown as FakeModel, 'cursor:12')

        retargetModel('/repo/attached-old.ts', '/repo/attached-new.ts')

        expect(editor.model?.uri.toString()).toBe('file:///repo/attached-new.ts')
        expect(editor.model?.isDisposed()).toBe(false)
        expect(editor.restoredViewStates).toEqual(['cursor:12'])
    })

    test('목적지에 남아있던 낡은 모델이 있어도 중복 uri 로 실패하지 않는다', async () => {
        const { getOrCreateModel, getModel, retargetModel } = await importModelRegistry()
        const stale = getOrCreateModel('/repo/stale-target.ts', 'stale content', 'plaintext')
        getOrCreateModel('/repo/live-source.ts', 'live content', 'typescript')

        retargetModel('/repo/live-source.ts', '/repo/stale-target.ts')

        expect(stale.isDisposed()).toBe(true)
        expect(getModel('/repo/stale-target.ts')?.getValue()).toBe('live content')
    })

    test('옮길 모델이 없으면 아무 일도 하지 않는다', async () => {
        const { getModel, retargetModel } = await importModelRegistry()

        retargetModel('/repo/never-opened.ts', '/repo/never-created.ts')

        expect(getModel('/repo/never-created.ts')).toBeUndefined()
        expect(fakeModels).toHaveLength(0)
    })
})

describe('applyModelLanguage', () => {
    test('확장자가 바뀐 뒤 새 languageId 를 모델에 반영한다', async () => {
        const { getOrCreateModel, getModel, retargetModel, applyModelLanguage } = await importModelRegistry()
        getOrCreateModel('/repo/notes.txt', 'text', 'plaintext')
        retargetModel('/repo/notes.txt', '/repo/notes.ts')

        applyModelLanguage('/repo/notes.ts', 'typescript')

        expect(getModel('/repo/notes.ts')?.getLanguageId()).toBe('typescript')
    })

    test('모델이 없거나 언어가 같으면 아무 일도 하지 않는다', async () => {
        const { getOrCreateModel, getModel, applyModelLanguage } = await importModelRegistry()
        getOrCreateModel('/repo/a.ts', 'body', 'typescript')

        applyModelLanguage('/repo/a.ts', 'typescript')
        applyModelLanguage('/repo/missing.ts', 'typescript')

        expect(getModel('/repo/a.ts')?.getLanguageId()).toBe('typescript')
        expect(getModel('/repo/missing.ts')).toBeUndefined()
    })
})

describe('disposeModel', () => {
    test('등록된 모델을 폐기하고 레지스트리에서 지운다', async () => {
        const { getOrCreateModel, getModel, disposeModel } = await importModelRegistry()
        const model = getOrCreateModel('/repo/closing.ts', 'body', 'typescript')

        disposeModel('/repo/closing.ts')

        expect(model.isDisposed()).toBe(true)
        expect(getModel('/repo/closing.ts')).toBeUndefined()
    })
})
