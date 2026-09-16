import { describe, expect, test } from 'bun:test'
import type { BuiltinTypeScriptDiagnosticsApi } from '@shared/lib/monaco/builtin-typescript'
import { BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS, configureBuiltinTypeScriptFallback } from '@shared/lib/monaco/builtin-typescript'

type RecordedOptions = Parameters<BuiltinTypeScriptDiagnosticsApi['typescriptDefaults']['setDiagnosticsOptions']>[0]

/**
 * Stands in for `monaco.typescript`. Real monaco cannot be imported here at all — its `?worker`
 * entries only resolve through Vite (`docs/memory/test-conventions.md` §3) — and this module takes
 * the language services as a parameter precisely so the fallback policy is assertable without it.
 */
const createFakeTypeScriptApi = () => {
    const typescriptCalls: RecordedOptions[] = []
    const javascriptCalls: RecordedOptions[] = []
    const api: BuiltinTypeScriptDiagnosticsApi = {
        typescriptDefaults: { setDiagnosticsOptions: (options) => typescriptCalls.push(options) },
        javascriptDefaults: { setDiagnosticsOptions: (options) => javascriptCalls.push(options) },
    }
    return { api, typescriptCalls, javascriptCalls }
}

describe('configureBuiltinTypeScriptFallback — 내장 ts worker 폴백 정직화 (d-64 F1)', () => {
    test('typescript·javascript 양쪽 language service 에 모두 적용한다', () => {
        const { api, typescriptCalls, javascriptCalls } = createFakeTypeScriptApi()

        configureBuiltinTypeScriptFallback(api)

        expect(typescriptCalls).toHaveLength(1)
        expect(javascriptCalls).toHaveLength(1)
    })

    test('semantic·suggestion 진단은 끄고 구문 검사만 남긴다', () => {
        const { api, typescriptCalls, javascriptCalls } = createFakeTypeScriptApi()

        configureBuiltinTypeScriptFallback(api)

        for (const options of [typescriptCalls[0], javascriptCalls[0]]) {
            expect(options?.noSemanticValidation).toBe(true)
            expect(options?.noSuggestionDiagnostics).toBe(true)
            expect(options?.noSyntaxValidation).toBe(false)
        }
    })

    test('전달하는 옵션은 공개 상수 그대로다 — 호출부가 임의 값을 만들지 않는다', () => {
        const { api, typescriptCalls } = createFakeTypeScriptApi()

        configureBuiltinTypeScriptFallback(api)

        expect(typescriptCalls[0]).toEqual(BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS)
    })
})
