import { describe, expect, test } from 'bun:test'
import type { BuiltinTypeScriptModeApi } from '@shared/lib/monaco/builtin-typescript-mode'
import {
    BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS,
    BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION,
    suspendBuiltinTypeScriptMode,
} from '@shared/lib/monaco/builtin-typescript-mode'

type ModeConfigurationArg = Parameters<BuiltinTypeScriptModeApi['typescriptDefaults']['setModeConfiguration']>[0]
type DiagnosticsOptionsArg = Parameters<BuiltinTypeScriptModeApi['typescriptDefaults']['setDiagnosticsOptions']>[0]

/**
 * A stand-in for one of monaco's built-in language services. Real monaco cannot be imported under
 * `bun test` (its `?worker` entries resolve only through Vite —
 * `docs/memory/test-conventions.md` §3), which is why `suspendBuiltinTypeScriptMode` takes the
 * services as a parameter. `modeConfiguration` is a plain mutable field here, mirroring the real
 * getter's observable behaviour: `setModeConfiguration` *replaces* the object rather than mutating
 * it (`esm/vs/languages/features/typescript/register.js`), which is what makes holding the previous
 * reference a valid snapshot.
 */
type FakeDefaults = {
    modeConfiguration: ModeConfigurationArg
    diagnosticsOptions: DiagnosticsOptionsArg
    modeConfigurationHistory: ModeConfigurationArg[]
    diagnosticsOptionsHistory: DiagnosticsOptionsArg[]
    setModeConfiguration: (next: ModeConfigurationArg) => void
    getDiagnosticsOptions: () => DiagnosticsOptionsArg
    setDiagnosticsOptions: (next: DiagnosticsOptionsArg) => void
}

const ORIGINAL_MODE_CONFIGURATION: ModeConfigurationArg = { completionItems: true, hovers: true, diagnostics: true }
const ORIGINAL_DIAGNOSTICS_OPTIONS: DiagnosticsOptionsArg = { noSemanticValidation: true, noSyntaxValidation: false, noSuggestionDiagnostics: true }

const createFakeDefaults = () => {
    const defaults: FakeDefaults = {
        modeConfiguration: ORIGINAL_MODE_CONFIGURATION,
        diagnosticsOptions: ORIGINAL_DIAGNOSTICS_OPTIONS,
        modeConfigurationHistory: [],
        diagnosticsOptionsHistory: [],
        setModeConfiguration: (next) => {
            defaults.modeConfiguration = next
            defaults.modeConfigurationHistory.push(next)
        },
        getDiagnosticsOptions: () => defaults.diagnosticsOptions,
        setDiagnosticsOptions: (next) => {
            defaults.diagnosticsOptions = next
            defaults.diagnosticsOptionsHistory.push(next)
        },
    }
    return defaults
}

const createFakeTypeScriptApi = () => {
    const typescriptDefaults = createFakeDefaults()
    const javascriptDefaults = createFakeDefaults()
    const api: BuiltinTypeScriptModeApi = { typescriptDefaults, javascriptDefaults }
    return { api, typescriptDefaults, javascriptDefaults }
}

describe('suspendBuiltinTypeScriptMode — LSP 세션 중 내장 provider 정지 (d-64 F2)', () => {
    test('정지하면 내장 기능을 전부 끄고 진단 3종을 모두 막는다', () => {
        const { api, typescriptDefaults } = createFakeTypeScriptApi()

        suspendBuiltinTypeScriptMode('typescript', api)

        expect(typescriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
        expect(typescriptDefaults.diagnosticsOptions).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS)
        expect(Object.values(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION).every((enabled) => enabled === false)).toBe(true)
    })

    test('세션이 하나라도 남아 있으면 복원하지 않는다 — 2회 획득 후 1회 해제', () => {
        const { api, typescriptDefaults } = createFakeTypeScriptApi()

        const releaseFirst = suspendBuiltinTypeScriptMode('typescript', api)
        suspendBuiltinTypeScriptMode('typescript', api)
        releaseFirst()

        expect(typescriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
        expect(typescriptDefaults.diagnosticsOptions).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS)
    })

    test('두 번째 정지는 스냅샷을 덮어쓰지 않는다 — 전부 해제하면 최초 상태로 돌아온다', () => {
        const { api, typescriptDefaults } = createFakeTypeScriptApi()

        const releaseFirst = suspendBuiltinTypeScriptMode('typescript', api)
        const releaseSecond = suspendBuiltinTypeScriptMode('typescript', api)
        releaseFirst()
        releaseSecond()

        expect(typescriptDefaults.modeConfiguration).toBe(ORIGINAL_MODE_CONFIGURATION)
        expect(typescriptDefaults.diagnosticsOptions).toBe(ORIGINAL_DIAGNOSTICS_OPTIONS)
    })

    test('같은 language service 를 공유하는 tsx 세션도 같은 refcount 에 합산된다', () => {
        const { api, typescriptDefaults, javascriptDefaults } = createFakeTypeScriptApi()

        const releaseTs = suspendBuiltinTypeScriptMode('typescript', api)
        const releaseTsx = suspendBuiltinTypeScriptMode('typescriptreact', api)
        releaseTs()

        expect(typescriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
        expect(javascriptDefaults.modeConfigurationHistory).toHaveLength(0)

        releaseTsx()
        expect(typescriptDefaults.modeConfiguration).toBe(ORIGINAL_MODE_CONFIGURATION)
    })

    test('javascript·javascriptreact 는 javascriptDefaults 만 정지한다', () => {
        const { api, typescriptDefaults, javascriptDefaults } = createFakeTypeScriptApi()

        const release = suspendBuiltinTypeScriptMode('javascriptreact', api)

        expect(javascriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
        expect(typescriptDefaults.modeConfigurationHistory).toHaveLength(0)
        expect(typescriptDefaults.diagnosticsOptionsHistory).toHaveLength(0)

        release()
        expect(javascriptDefaults.modeConfiguration).toBe(ORIGINAL_MODE_CONFIGURATION)
    })

    test('TS/JS 가 아닌 언어는 아무것도 건드리지 않는다 — release 도 무해하다', () => {
        const { api, typescriptDefaults, javascriptDefaults } = createFakeTypeScriptApi()

        const release = suspendBuiltinTypeScriptMode('rust', api)
        release()

        expect(typescriptDefaults.modeConfigurationHistory).toHaveLength(0)
        expect(typescriptDefaults.diagnosticsOptionsHistory).toHaveLength(0)
        expect(javascriptDefaults.modeConfigurationHistory).toHaveLength(0)
        expect(javascriptDefaults.diagnosticsOptionsHistory).toHaveLength(0)
    })

    test('release 를 두 번 호출해도 다른 세션의 refcount 를 깎지 않는다', () => {
        const { api, typescriptDefaults } = createFakeTypeScriptApi()

        const releaseFirst = suspendBuiltinTypeScriptMode('typescript', api)
        suspendBuiltinTypeScriptMode('typescript', api)
        releaseFirst()
        releaseFirst()

        expect(typescriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
        expect(typescriptDefaults.diagnosticsOptions).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS)
    })

    test('해제 후 다시 정지하면 복원된 상태를 새 스냅샷으로 잡는다', () => {
        const { api, typescriptDefaults } = createFakeTypeScriptApi()

        suspendBuiltinTypeScriptMode('typescript', api)()
        const release = suspendBuiltinTypeScriptMode('typescript', api)
        expect(typescriptDefaults.modeConfiguration).toEqual(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)

        release()
        expect(typescriptDefaults.modeConfiguration).toBe(ORIGINAL_MODE_CONFIGURATION)
        expect(typescriptDefaults.diagnosticsOptions).toBe(ORIGINAL_DIAGNOSTICS_OPTIONS)
    })
})
