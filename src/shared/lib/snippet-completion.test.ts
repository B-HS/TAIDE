import { describe, expect, test } from 'bun:test'
import type { SnippetFile } from '@shared/api/bindings'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import {
    collectSnippetCompletionCandidates,
    registerSnippetCompletions,
    registerSnippetCompletionsForLanguages,
} from '@shared/lib/snippet-completion'
import { TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'

const languageFile = (fileName: string, snippets: SnippetFile['snippets']): SnippetFile => ({ fileName, snippets })

describe('collectSnippetCompletionCandidates', () => {
    test('<languageId>.json 파일은 그 언어에서만 후보를 반환한다', () => {
        const files = [languageFile('typescript.json', { 'For loop': { prefix: 'for', body: 'for (;;) {}' } })]
        expect(collectSnippetCompletionCandidates(files, 'typescript')).toEqual([
            { name: 'For loop', prefix: 'for', body: 'for (;;) {}', description: undefined },
        ])
        expect(collectSnippetCompletionCandidates(files, 'javascript')).toEqual([])
    })

    test('body 배열과 description 배열 모두 개행으로 join 된다 (VS Code 의 string[] 필드 의미론)', () => {
        const files = [
            languageFile('rust.json', {
                Main: { prefix: 'main', body: ['fn main() {', '\t$0', '}'], description: ['entry', 'point'] },
            }),
        ]
        expect(collectSnippetCompletionCandidates(files, 'rust')).toEqual([
            { name: 'Main', prefix: 'main', body: 'fn main() {\n\t$0\n}', description: 'entry\npoint' },
        ])
    })

    test('prefix 배열은 후보마다 하나씩 분리된 완성 항목을 만든다', () => {
        const files = [languageFile('python.json', { Print: { prefix: ['pr', 'print'], body: 'print($0)' } })]
        expect(collectSnippetCompletionCandidates(files, 'python')).toEqual([
            { name: 'Print', prefix: 'pr', body: 'print($0)', description: undefined },
            { name: 'Print', prefix: 'print', body: 'print($0)', description: undefined },
        ])
    })

    test('.code-snippets 파일은 scope 가 없으면 모든 언어에 매칭된다', () => {
        const files = [languageFile('global.code-snippets', { Todo: { prefix: 'todo', body: '// TODO: $0' } })]
        expect(collectSnippetCompletionCandidates(files, 'go')).toHaveLength(1)
        expect(collectSnippetCompletionCandidates(files, 'markdown')).toHaveLength(1)
    })

    test('.code-snippets 파일의 scope 는 콤마로 나열된 languageId 만 매칭한다', () => {
        const files = [
            languageFile('global.code-snippets', {
                'FE only': { prefix: 'fe', body: 'fe', scope: 'typescript,typescriptreact' },
            }),
        ]
        expect(collectSnippetCompletionCandidates(files, 'typescriptreact')).toHaveLength(1)
        expect(collectSnippetCompletionCandidates(files, 'go')).toHaveLength(0)
    })

    test('<languageId>.json 파일의 scope 필드는 무시된다 (code-snippets 전용)', () => {
        const files = [languageFile('go.json', { Scoped: { prefix: 'x', body: 'x', scope: 'rust' } })]
        expect(collectSnippetCompletionCandidates(files, 'go')).toHaveLength(1)
    })

    test('언어별 파일과 전역 파일을 함께 합산한다', () => {
        const files = [
            languageFile('go.json', { A: { prefix: 'a', body: 'a' } }),
            languageFile('global.code-snippets', { B: { prefix: 'b', body: 'b' } }),
            languageFile('rust.json', { C: { prefix: 'c', body: 'c' } }),
        ]
        expect(collectSnippetCompletionCandidates(files, 'go').map((candidate) => candidate.name)).toEqual(['A', 'B'])
    })
})

type CompletionProviderArg = Parameters<Monaco['languages']['registerCompletionItemProvider']>[1]

const createFakeMonaco = () => {
    const providersByLanguageId = new Map<string, CompletionProviderArg>()
    let disposeCallCount = 0
    const fakeMonaco = {
        Range: class {
            constructor(
                public startLineNumber: number,
                public startColumn: number,
                public endLineNumber: number,
                public endColumn: number,
            ) {}
        },
        languages: {
            CompletionItemKind: { Snippet: 27 },
            CompletionItemInsertTextRule: { InsertAsSnippet: 4 },
            registerCompletionItemProvider: (languageId: string, provider: CompletionProviderArg) => {
                providersByLanguageId.set(languageId, provider)
                return { dispose: () => (disposeCallCount += 1) }
            },
        },
    }
    return { monaco: fakeMonaco as unknown as Monaco, providersByLanguageId, getDisposeCallCount: () => disposeCallCount }
}

const fakeModel = {
    getWordUntilPosition: () => ({ startColumn: 1, endColumn: 1 }),
} as unknown as Parameters<NonNullable<CompletionProviderArg['provideCompletionItems']>>[0]

const fakePosition = { lineNumber: 1, column: 1 } as Parameters<NonNullable<CompletionProviderArg['provideCompletionItems']>>[1]

describe('registerSnippetCompletions', () => {
    test('TAIDE_LANGUAGE_IDS 각각에 대해 정확한 languageId 로 provider 를 등록한다 (* 금지)', () => {
        const { monaco, providersByLanguageId } = createFakeMonaco()
        registerSnippetCompletions(monaco, { getSnippetFiles: () => [] })
        expect([...providersByLanguageId.keys()].toSorted()).toEqual([...TAIDE_LANGUAGE_IDS].toSorted())
        expect(providersByLanguageId.has('*')).toBe(false)
    })

    test('provider 는 kind=Snippet·InsertAsSnippet·documentation=description 으로 항목을 만든다', async () => {
        const { monaco, providersByLanguageId } = createFakeMonaco()
        registerSnippetCompletions(monaco, {
            getSnippetFiles: () => [languageFile('go.json', { Main: { prefix: 'main', body: 'func main() {}', description: 'entrypoint' } })],
        })

        const provider = providersByLanguageId.get('go')
        const result = await provider?.provideCompletionItems(fakeModel, fakePosition, {} as never, {} as never)
        expect(result).toMatchObject({
            suggestions: [
                {
                    label: 'main',
                    kind: 27,
                    insertText: 'func main() {}',
                    insertTextRules: 4,
                    documentation: 'entrypoint',
                    detail: 'Main',
                },
            ],
        })
    })

    test('dispose 는 등록된 모든 언어 provider 를 해제한다', () => {
        const { monaco, getDisposeCallCount } = createFakeMonaco()
        const disposable = registerSnippetCompletions(monaco, { getSnippetFiles: () => [] })
        disposable.dispose()
        expect(getDisposeCallCount()).toBe(TAIDE_LANGUAGE_IDS.length)
    })
})

describe('registerSnippetCompletionsForLanguages — 플러그인 언어 스니펫 provider (audit §4-B D6)', () => {
    test('설치 이후 등록된 플러그인 언어에도 provider 를 붙인다(재현: 플러그인 언어에는 스니펫 완성 전무)', () => {
        const { monaco, providersByLanguageId } = createFakeMonaco()
        const disposable = registerSnippetCompletions(monaco, {
            getSnippetFiles: () => [languageFile('mylang.json', { Hello: { prefix: 'hi', body: 'hello' } })],
        })
        expect(providersByLanguageId.has('mylang')).toBe(false)

        registerSnippetCompletionsForLanguages(['mylang'])

        expect(providersByLanguageId.has('mylang')).toBe(true)
        disposable.dispose()
    })

    test('같은 언어를 다시 등록해도 provider 를 중복 생성하지 않는다', () => {
        const { monaco } = createFakeMonaco()
        let registerCallCount = 0
        const countingMonaco = {
            ...monaco,
            languages: {
                ...monaco.languages,
                registerCompletionItemProvider: (...args: Parameters<Monaco['languages']['registerCompletionItemProvider']>) => {
                    if (args[0] === 'mylang') registerCallCount += 1
                    return monaco.languages.registerCompletionItemProvider(...args)
                },
            },
        } as unknown as Monaco

        const disposable = registerSnippetCompletions(countingMonaco, { getSnippetFiles: () => [] })
        registerSnippetCompletionsForLanguages(['mylang'])
        registerSnippetCompletionsForLanguages(['mylang'])

        expect(registerCallCount).toBe(1)
        disposable.dispose()
    })

    test('설치 전에 요청된 플러그인 언어도 이후 설치에서 함께 등록된다(부트스트랩 순서 무관)', () => {
        registerSnippetCompletionsForLanguages(['early-lang'])
        const { monaco, providersByLanguageId } = createFakeMonaco()

        const disposable = registerSnippetCompletions(monaco, { getSnippetFiles: () => [] })

        expect(providersByLanguageId.has('early-lang')).toBe(true)
        disposable.dispose()
    })
})
