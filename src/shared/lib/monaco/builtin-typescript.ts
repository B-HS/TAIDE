import type { Monaco } from '@shared/lib/lsp/monaco-types'

type LanguageServiceDefaults = Monaco['typescript']['typescriptDefaults']

/**
 * The slice of monaco's built-in TypeScript/JavaScript language services this module touches.
 *
 * Note the namespace: in monaco-editor 0.56 the defaults live on the *top-level* `monaco.typescript`
 * (`node_modules/monaco-editor/esm/vs/index.js:90-91` re-exports
 * `languages/features/typescript/register.js` as `typescript`), while the historical
 * `monaco.languages.typescript` survives only as a type-only `{ deprecated: true }` marker in
 * `monaco.d.ts` with no runtime value behind it.
 *
 * Narrowed to the single method used so a unit test can hand this function a plain recorder object
 * instead of booting real monaco (whose `?worker` imports `bun test` cannot resolve at all).
 */
export type BuiltinTypeScriptDiagnosticsApi = {
    typescriptDefaults: Pick<LanguageServiceDefaults, 'setDiagnosticsOptions'>
    javascriptDefaults: Pick<LanguageServiceDefaults, 'setDiagnosticsOptions'>
}

/**
 * Syntax errors only. The built-in ts worker is TAIDE's fallback for when no TypeScript language
 * server session is attached, and in that role it has no project context whatsoever — no
 * `tsconfig.json` (nothing in this repo ever called `setCompilerOptions`), therefore classic module
 * resolution, and no `node_modules` on any filesystem it can reach. Every semantic diagnostic it
 * produces about a real project is consequently a false positive (d-64 §0.1 measured TS2792
 * "Did you mean to set the 'moduleResolution' option to 'nodenext'" and TS2580 "Cannot find name
 * 'process'" on a file whose imports all resolve fine under the project's own tsconfig), so
 * semantic and suggestion diagnostics are turned off and syntactic validation — which needs no
 * project context to be correct — is kept.
 *
 * Diagnostics are the only built-in capability this disables: completion, hover and the rest stay
 * on, because a lib-only language service is still the best answer available while no server is
 * attached. Suspending those belongs to `suspendBuiltinTypeScriptMode`
 * (`builtin-typescript-mode.ts`), which only acts while a session actually is attached.
 */
export const BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS = {
    noSemanticValidation: true,
    noSyntaxValidation: false,
    noSuggestionDiagnostics: true,
} as const

/**
 * Applies {@link BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS} to both built-in language
 * services. Called once from `setup.ts` at import time, before any model exists.
 *
 * `javascriptDefaults` already ships with `noSemanticValidation: true`
 * (`esm/vs/languages/features/typescript/register.js`), but it is configured here anyway rather
 * than relying on that default: `setDiagnosticsOptions` replaces the whole options object, so
 * stating both services explicitly keeps the fallback policy readable in one place and immune to an
 * upstream default flip.
 */
export const configureBuiltinTypeScriptFallback = (typescriptApi: BuiltinTypeScriptDiagnosticsApi) => {
    typescriptApi.typescriptDefaults.setDiagnosticsOptions(BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS)
    typescriptApi.javascriptDefaults.setDiagnosticsOptions(BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS)
}
