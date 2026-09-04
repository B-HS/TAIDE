import { describe, expect, test } from 'bun:test'
import type { AiInlineCompleteRequest, AiTextResponse, AiTokenStatus } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import type { AiInlineCompletionClient, AiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { acquireAiInlineCompletionProvider, resolveAiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'

/**
 * Auto-tab's monaco provider. Everything here runs against a hand-built `ITextModel`/monaco stub
 * rather than a real editor (`docs/memory/test-conventions.md` §5 — monaco needs a real DOM), which
 * is enough because the module's decisions are all made before monaco is touched: which owner is
 * live, whether the cache already answers, whether the request was cancelled, and what the
 * response is turned into.
 *
 * The completion cache is module scope and shared across every test in this process, so each test
 * uses its own file path — the cache key starts with it.
 */
type FakeMonaco = {
    instance: typeof monaco
    registrations: monaco.languages.InlineCompletionsProvider[]
    disposeCounts: () => number
}

const createFakeMonaco = (): FakeMonaco => {
    const registrations: monaco.languages.InlineCompletionsProvider[] = []
    let disposeCount = 0
    const instance = {
        languages: {
            InlineCompletionTriggerKind: { Automatic: 0, Explicit: 1 },
            registerInlineCompletionsProvider: (_selector: unknown, provider: monaco.languages.InlineCompletionsProvider) => {
                registrations.push(provider)
                return {
                    dispose: () => {
                        disposeCount += 1
                    },
                }
            },
        },
    }

    return { instance: instance as unknown as typeof monaco, registrations, disposeCounts: () => disposeCount }
}

/** A single-line model: offsets and columns differ by one, which is all `buildContextWindow` needs. */
const createFakeModel = (text: string, path: string, languageId = 'typescript') =>
    ({
        uri: { path },
        getLanguageId: () => languageId,
        getValueLength: () => text.length,
        getOffsetAt: (position: monaco.Position) => position.column - 1,
        getPositionAt: (offset: number) => ({ lineNumber: 1, column: offset + 1 }),
        getValueInRange: (range: monaco.IRange) => text.slice(range.startColumn - 1, range.endColumn - 1),
    }) as unknown as monaco.editor.ITextModel

const createPosition = (column: number) => ({ lineNumber: 1, column }) as unknown as monaco.Position

type FakeToken = monaco.CancellationToken & { cancel: () => void }

const createFakeToken = (): FakeToken => {
    const listeners: (() => void)[] = []
    const token = {
        isCancellationRequested: false,
        onCancellationRequested: (listener: () => void) => {
            listeners.push(listener)
            return { dispose: () => undefined }
        },
        cancel: () => {
            token.isCancellationRequested = true
            for (const listener of listeners) listener()
        },
    }

    return token as unknown as FakeToken
}

const explicitContext = { triggerKind: 1 } as unknown as monaco.languages.InlineCompletionContext
const automaticContext = { triggerKind: 0 } as unknown as monaco.languages.InlineCompletionContext

type CompleteRequest = Omit<AiInlineCompleteRequest, 'owner'>

const createFakeClient = (respond: (request: CompleteRequest) => Promise<AiTextResponse>) => {
    const requests: CompleteRequest[] = []
    const cancelled: string[] = []
    const client: AiInlineCompletionClient = {
        complete: (request) => {
            requests.push(request)
            return respond(request)
        },
        cancel: (requestId) => {
            cancelled.push(requestId)
            return Promise.resolve(null)
        },
    }

    return { client, requests, cancelled }
}

const CONFIG: AiInlineCompletionConfig = { provider: 'codex', model: 'gpt-5-codex' }

const textResponse = (text: string) => ({ text }) as unknown as AiTextResponse

const provideOnce = (provider: monaco.languages.InlineCompletionsProvider, model: monaco.editor.ITextModel, column: number, token: FakeToken) =>
    provider.provideInlineCompletions(model, createPosition(column), explicitContext, token)

describe('resolveAiInlineCompletionConfig', () => {
    const tokenStatus = { codex: true, omlx: false } as unknown as AiTokenStatus

    test('provider·model·토큰이 모두 갖춰지면 쌍을 돌려준다', () => {
        expect(resolveAiInlineCompletionConfig({ aiProvider: 'codex', aiModel: 'gpt-5-codex' }, tokenStatus)).toEqual({
            provider: 'codex',
            model: 'gpt-5-codex',
        })
    })

    test('provider 나 model 이 비면 null 이다', () => {
        expect(resolveAiInlineCompletionConfig({ aiProvider: null, aiModel: 'gpt-5-codex' }, tokenStatus)).toBeNull()
        expect(resolveAiInlineCompletionConfig({ aiProvider: 'codex', aiModel: null }, tokenStatus)).toBeNull()
        expect(resolveAiInlineCompletionConfig({ aiProvider: 'codex', aiModel: '' }, tokenStatus)).toBeNull()
    })

    test('설정 자체가 아직 없으면 null 이다', () => {
        expect(resolveAiInlineCompletionConfig(undefined, tokenStatus)).toBeNull()
    })

    test('선택한 provider 의 토큰이 지워졌으면 null 이다 (토큰 상태 미조회 포함)', () => {
        expect(resolveAiInlineCompletionConfig({ aiProvider: 'omlx', aiModel: 'gpt-5' }, tokenStatus)).toBeNull()
        expect(resolveAiInlineCompletionConfig({ aiProvider: 'codex', aiModel: 'gpt-5-codex' }, undefined)).toBeNull()
    })
})

describe('acquireAiInlineCompletionProvider — 소유권', () => {
    test('여러 pane 이 획득해도 monaco 에는 한 번만 등록한다', () => {
        const fakeMonaco = createFakeMonaco()
        const { client } = createFakeClient(() => Promise.resolve(textResponse('')))

        const releaseFirst = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)
        const releaseSecond = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        expect(fakeMonaco.registrations.length).toBe(1)

        releaseFirst()
        releaseSecond()
    })

    test('마지막 소유자가 놓을 때만 등록을 dispose 한다', () => {
        const fakeMonaco = createFakeMonaco()
        const { client } = createFakeClient(() => Promise.resolve(textResponse('')))

        const releaseFirst = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)
        const releaseSecond = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        releaseSecond()
        expect(fakeMonaco.disposeCounts()).toBe(0)

        releaseFirst()
        expect(fakeMonaco.disposeCounts()).toBe(1)
    })

    test('요청은 가장 최근에 획득한 소유자의 client 로 간다', async () => {
        const fakeMonaco = createFakeMonaco()
        const first = createFakeClient(() => Promise.resolve(textResponse('first')))
        const second = createFakeClient(() => Promise.resolve(textResponse('second')))

        const releaseFirst = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, first.client)
        const releaseSecond = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, second.client)
        await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/owner-latest.ts'), 6, createFakeToken())

        expect(first.requests.length).toBe(0)
        expect(second.requests.length).toBe(1)

        releaseSecond()
        releaseFirst()
    })

    test('최근 소유자가 언마운트되면 그 앞 소유자로 되돌아간다', async () => {
        const fakeMonaco = createFakeMonaco()
        const first = createFakeClient(() => Promise.resolve(textResponse('first')))
        const second = createFakeClient(() => Promise.resolve(textResponse('second')))

        const releaseFirst = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, first.client)
        const releaseSecond = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, second.client)
        releaseSecond()
        await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/owner-fallback.ts'), 6, createFakeToken())

        expect(first.requests.length).toBe(1)

        releaseFirst()
    })

    test('놓은 뒤 다시 획득하면 새 등록을 만든다', () => {
        const fakeMonaco = createFakeMonaco()
        const { client } = createFakeClient(() => Promise.resolve(textResponse('')))

        acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)()
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        expect(fakeMonaco.registrations.length).toBe(2)

        release()
    })
})

describe('provideInlineCompletions', () => {
    test('설정이 없으면 요청 없이 null 이다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('x')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => null, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/no-config.ts'), 6, createFakeToken())

        expect(result).toBeNull()
        expect(requests.length).toBe(0)

        release()
    })

    test('커서 기준 앞뒤 문맥·언어·파일 경로를 실어 보내고 응답을 커서 위치 아이템으로 감싼다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('suggested')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/context.ts'), 10, createFakeToken())

        expect(requests[0].prefix).toBe('const a =')
        expect(requests[0].suffix).toBe(' 1')
        expect(requests[0].language).toBe('typescript')
        expect(requests[0].filePath).toBe('/context.ts')
        expect(requests[0].provider).toBe(CONFIG.provider)
        expect(requests[0].model).toBe(CONFIG.model)
        expect(result?.items).toEqual([{ insertText: 'suggested', range: { startLineNumber: 1, startColumn: 10, endLineNumber: 1, endColumn: 10 } }])

        release()
    })

    test('같은 문맥을 다시 물으면 캐시로 답하고 두 번째 요청을 보내지 않는다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('cached')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)
        const model = createFakeModel('const a = 1', '/cache-hit.ts')

        await provideOnce(fakeMonaco.registrations[0], model, 6, createFakeToken())
        const second = await provideOnce(fakeMonaco.registrations[0], model, 6, createFakeToken())

        expect(requests.length).toBe(1)
        expect(second?.items[0].insertText).toBe('cached')

        release()
    })

    test('문맥이 달라지면 캐시가 적중하지 않는다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('cached')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)
        const model = createFakeModel('const a = 1', '/cache-miss.ts')

        await provideOnce(fakeMonaco.registrations[0], model, 6, createFakeToken())
        await provideOnce(fakeMonaco.registrations[0], model, 7, createFakeToken())

        expect(requests.length).toBe(2)

        release()
    })

    test('빈 응답은 캐시하지 않고 null 을 돌려준다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)
        const model = createFakeModel('const a = 1', '/empty-response.ts')

        const result = await provideOnce(fakeMonaco.registrations[0], model, 6, createFakeToken())
        await provideOnce(fakeMonaco.registrations[0], model, 6, createFakeToken())

        expect(result).toBeNull()
        expect(requests.length).toBe(2)

        release()
    })

    test('요청이 실패해도 예외를 밖으로 내지 않고 null 이다 (에디터 타이핑 방해 금지)', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client } = createFakeClient(() => Promise.reject(new Error('provider down')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/failure.ts'), 6, createFakeToken())

        expect(result).toBeNull()

        release()
    })

    test('요청 도중 취소되면 결과를 버리고 백엔드에 취소를 알린다', async () => {
        const fakeMonaco = createFakeMonaco()
        const token = createFakeToken()
        const { client, requests, cancelled } = createFakeClient(async () => {
            token.cancel()
            return textResponse('too late')
        })
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/cancel-inflight.ts'), 6, token)

        expect(result).toBeNull()
        expect(cancelled).toEqual([requests[0].requestId])

        release()
    })

    test('자동 트리거는 디바운스 중이며, 그 사이 취소되면 요청을 아예 보내지 않는다', async () => {
        const fakeMonaco = createFakeMonaco()
        const token = createFakeToken()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('never')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const pending = fakeMonaco.registrations[0].provideInlineCompletions(
            createFakeModel('const a = 1', '/debounce-cancel.ts'),
            createPosition(6),
            automaticContext,
            token,
        )
        token.cancel()

        expect(await pending).toBeNull()
        expect(requests.length).toBe(0)

        release()
    })

    test('명시 트리거는 디바운스를 건너뛰고 즉시 요청한다', async () => {
        const fakeMonaco = createFakeMonaco()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('now')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/explicit.ts'), 6, createFakeToken())

        expect(requests.length).toBe(1)
        expect(result?.items[0].insertText).toBe('now')

        release()
    })

    test('이미 취소된 토큰으로 들어오면 요청하지 않는다', async () => {
        const fakeMonaco = createFakeMonaco()
        const token = createFakeToken()
        token.cancel()
        const { client, requests } = createFakeClient(() => Promise.resolve(textResponse('never')))
        const release = acquireAiInlineCompletionProvider(fakeMonaco.instance, () => CONFIG, client)

        const result = await provideOnce(fakeMonaco.registrations[0], createFakeModel('const a = 1', '/already-cancelled.ts'), 6, token)

        expect(result).toBeNull()
        expect(requests.length).toBe(0)

        release()
    })
})
