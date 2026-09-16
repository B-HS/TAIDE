import type { Monaco } from '@shared/lib/lsp/monaco-types'

type LanguageServiceDefaults = Monaco['typescript']['typescriptDefaults']
type ModeConfiguration = LanguageServiceDefaults['modeConfiguration']
type DiagnosticsOptions = ReturnType<LanguageServiceDefaults['getDiagnosticsOptions']>

/**
 * One built-in language service (`typescriptDefaults` / `javascriptDefaults`), narrowed to the
 * four members this module reads and writes so a unit test can hand it a plain fake.
 */
type SuspendableDefaults = Pick<
    LanguageServiceDefaults,
    'modeConfiguration' | 'setModeConfiguration' | 'getDiagnosticsOptions' | 'setDiagnosticsOptions'
>

/**
 * Both built-in language services — `monaco.typescript` in monaco-editor 0.56, see
 * `builtin-typescript.ts`'s note on that namespace move.
 */
export type BuiltinTypeScriptModeApi = {
    typescriptDefaults: SuspendableDefaults
    javascriptDefaults: SuspendableDefaults
}

/**
 * Every built-in feature off. Typed `Required<ModeConfiguration>` rather than `ModeConfiguration`
 * on purpose: every field of monaco's interface is optional, so a plain annotation would silently
 * accept a partial object (a missing field reads as `undefined`, i.e. falsy, today — but it would
 * also silently miss a *new* capability a monaco upgrade adds). `Required` makes leaving one out a
 * typecheck failure. The 13 fields mirror `ModeConfiguration` in
 * `node_modules/monaco-editor/monaco.d.ts`.
 */
export const BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION: Required<ModeConfiguration> = {
    completionItems: false,
    hovers: false,
    documentSymbols: false,
    definitions: false,
    references: false,
    documentHighlights: false,
    rename: false,
    diagnostics: false,
    documentRangeFormattingEdits: false,
    signatureHelp: false,
    onTypeFormattingEdits: false,
    codeActions: false,
    inlayHints: false,
}

/**
 * All three validations off, including the syntactic one `configureBuiltinTypeScriptFallback` keeps
 * enabled for the no-session fallback: while a real language server owns the language it also
 * reports syntax errors, and both sets of markers would otherwise be drawn at once under two
 * different monaco marker owners (`'typescript'` for the built-in worker, `lsp-<serverId>-<n>` for
 * the session — `adapters/diagnostics.ts`).
 */
export const BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS = {
    noSemanticValidation: true,
    noSyntaxValidation: true,
    noSuggestionDiagnostics: true,
} as const

type SuspensionState = {
    count: number
    modeConfiguration: ModeConfiguration
    diagnosticsOptions: DiagnosticsOptions
}

/**
 * Refcount plus the pre-suspension snapshot, keyed by the *defaults object* rather than by language
 * id: `typescript` and `typescriptreact` share one language service, so a session on either must
 * count against the same suspension. Weak so a test's throwaway fake is not retained.
 */
const suspensionByDefaults = new WeakMap<SuspendableDefaults, SuspensionState>()

/**
 * Which built-in language service a TAIDE language id belongs to. `typescriptreact` /
 * `javascriptreact` are TAIDE-only ids (`shared/lib/shiki/lang-map.ts`) that monaco's built-in
 * worker never attaches to — it only ever sees the `typescript` / `javascript` ids — but they map
 * here anyway because the suspension is keyed off the language service a session claims, and a
 * `.tsx` session claiming the TypeScript service must still suspend it for the `.ts` models sharing
 * it. Every other language id is absent, which is what makes this a no-op for them.
 */
const DEFAULTS_KEY_BY_LANGUAGE_ID = new Map<string, keyof BuiltinTypeScriptModeApi>([
    ['typescript', 'typescriptDefaults'],
    ['typescriptreact', 'typescriptDefaults'],
    ['javascript', 'javascriptDefaults'],
    ['javascriptreact', 'javascriptDefaults'],
])

const NO_OP_RELEASE = () => {}

const acquireSuspension = (defaults: SuspendableDefaults) => {
    const existing = suspensionByDefaults.get(defaults)
    if (existing) {
        existing.count += 1
        return
    }
    suspensionByDefaults.set(defaults, {
        count: 1,
        modeConfiguration: defaults.modeConfiguration,
        diagnosticsOptions: defaults.getDiagnosticsOptions(),
    })
    defaults.setModeConfiguration(BUILTIN_TYPESCRIPT_SUSPENDED_MODE_CONFIGURATION)
    defaults.setDiagnosticsOptions(BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS)
}

const releaseSuspension = (defaults: SuspendableDefaults) => {
    const state = suspensionByDefaults.get(defaults)
    if (!state) return
    state.count -= 1
    if (state.count > 0) return
    suspensionByDefaults.delete(defaults)
    defaults.setModeConfiguration(state.modeConfiguration)
    defaults.setDiagnosticsOptions(state.diagnosticsOptions)
}

/**
 * Suspends monaco's built-in TypeScript/JavaScript language service for as long as an LSP session
 * owns `languageId`, and returns the `release` that restores it. Called from
 * `ensureLanguageRegistered` (`entities/lsp/lsp-session-registry.ts`), which parks the returned
 * function in that language's `disposables` array so `disposeSession` restores the built-in service
 * automatically when the last session for the language goes away.
 *
 * Refcounted per language service because several sessions can own it at once — two projects, or a
 * `.ts` and a `.tsx` pane, each register the same service once. Only the 0 -> 1 transition snapshots
 * and suspends; only 1 -> 0 restores the snapshot, so an intermediate release never resurrects the
 * built-in providers under a session that is still running. The returned release is idempotent: a
 * second call is ignored, so a double `dispose()` cannot decrement another owner's count.
 *
 * What each half actually achieves in monaco-editor 0.56 (verified against
 * `node_modules/monaco-editor/esm/vs/languages/features/typescript/`):
 *
 * - `setDiagnosticsOptions` takes effect immediately. `DiagnosticsAdapter` subscribes to
 *   `defaults.onDidChange` (`languageFeatures.js:183`) and reacts by clearing every model's markers
 *   through `onModelRemoved` -> `editor.setModelMarkers(model, selector, [])` and re-validating
 *   under the new options (`languageFeatures.js:153`, `:204-212`). This is the half that
 *   removes the duplicate/false squiggles the moment a session attaches.
 * - `setModeConfiguration` does *not* retroactively unregister providers in this version.
 *   `setupMode` reads `modeConfiguration` inside `registerProviders()` and calls it exactly once,
 *   with no `defaults.onDidChange` re-registration (`tsMode.js:30-132`) — so completion/hover/etc.
 *   providers already registered stay registered. It is still set here because it is the API
 *   contract for "built-in features off" (any mode set up *after* this point registers nothing),
 *   and because the snapshot/restore pair keeps the state honest either way. Fully unregistering
 *   already-live providers would need an upstream change; `docs/features/editor.md` §12 records it
 *   as a known limit rather than faking it here.
 *
 * Any language id outside {@link DEFAULTS_KEY_BY_LANGUAGE_ID} is a no-op with a release that does
 * nothing — a Rust or Python session must not touch the TypeScript service.
 */
export const suspendBuiltinTypeScriptMode = (languageId: string, typescriptApi: BuiltinTypeScriptModeApi) => {
    const defaultsKey = DEFAULTS_KEY_BY_LANGUAGE_ID.get(languageId)
    if (!defaultsKey) return NO_OP_RELEASE

    const defaults = typescriptApi[defaultsKey]
    acquireSuspension(defaults)

    let isReleased = false
    return () => {
        if (isReleased) return
        isReleased = true
        releaseSuspension(defaults)
    }
}
