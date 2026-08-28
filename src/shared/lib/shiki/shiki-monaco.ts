import { createHighlighterCore, type HighlighterCore, type LanguageRegistration, type ThemeRegistrationAny } from '@shikijs/core'
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript'
import { shikiToMonaco } from '@shikijs/monaco'
import type { ResolvedTheme } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { buildMonacoThemeData, TAIDE_MONACO_THEME_NAME } from '@shared/lib/monaco/theme'
import { buildShikiTheme } from '@shared/lib/shiki/build-shiki-theme'
import { loadAllTaideGrammars } from '@shared/lib/shiki/lang-map'
import { createLeadingTrailingDebouncer } from '@shared/lib/leading-trailing-debouncer'

const PLACEHOLDER_SHIKI_THEME: ThemeRegistrationAny = { name: TAIDE_MONACO_THEME_NAME, type: 'dark', colors: {}, tokenColors: [] }

/**
 * A debounce window picked to collapse a still-dragging color-picker burst to a single trailing
 * re-apply while staying short enough to avoid a perceptible "the editor stopped updating" lag — not
 * a measured monaco retokenize cost; that cost has not actually been benchmarked (contract d-45 F-08).
 * `applyShikiTheme`'s real work (`highlighter.loadTheme` + monaco tokens-provider repatch, see
 * {@link attachShikiTokensProvider}) is expensive enough that running it once per `pointermove` during
 * a theme editor color-picker drag floods the main thread — the same class of problem the per-move
 * native window-appearance IPC caused before it was gated (contract d-45 §0).
 */
const SHIKI_THEME_REAPPLY_DEBOUNCE_MS = 150

type ShikiMonacoNamespace = Parameters<typeof shikiToMonaco>[1]

type MonacoEditorSetTheme = typeof monaco.editor.setTheme
type MonacoEditorCreate = typeof monaco.editor.create

let highlighter: HighlighterCore | null = null
let lastResolvedTheme: ResolvedTheme | null = null
let originalSetTheme: MonacoEditorSetTheme | null = null
let originalCreate: MonacoEditorCreate | null = null

/**
 * Serializes every `initShiki`/`reinitShiki` body — and, since contract d-45 §1's review of
 * `applyShikiTheme`, every `applyShikiTheme` re-apply too — behind one FIFO chain (F6#19). Without
 * this, two concurrent calls (e.g. a plugin-reload-triggered `reinitShiki` racing an in-flight
 * `initShiki`, or an `applyShikiTheme` re-apply landing mid-`reinitShiki`) could each run
 * `captureOriginalMonacoEditorApi` → `createHighlighterCore` → `attachShikiTokensProvider`'s
 * `restoreOriginalMonacoEditorApi` → `shikiToMonaco` interleaved: `shikiToMonaco` monkeypatches
 * `monaco.editor.setTheme`/`create`, and two overlapping restore-then-repatch sequences can leave
 * monaco's public API wrapped around a highlighter that call B's own `reinitShiki` has since
 * disposed, or leave call A's finished highlighter's registration silently clobbered by call B's
 * before call A ever disposed its own stale one; `applyShikiTheme`'s fallback path
 * (`applyFallbackMonacoTheme`) reads `monaco.editor.setTheme` too, so it can observe the same
 * mid-monkeypatch state if it runs outside this queue. Every operation queued here runs to
 * completion before the next one starts, so "restore original → repatch with the new highlighter →
 * (reinit only) dispose the stale one" is always atomic relative to any other call.
 */
let operationQueue: Promise<void> = Promise.resolve()

const runExclusive = (operation: () => Promise<void>): Promise<void> => {
    const scheduled = operationQueue.then(operation, operation)
    operationQueue = scheduled.then(
        () => undefined,
        () => undefined,
    )
    return scheduled
}

const captureOriginalMonacoEditorApi = () => {
    if (originalSetTheme && originalCreate) return
    originalSetTheme = monaco.editor.setTheme
    originalCreate = monaco.editor.create
}

const restoreOriginalMonacoEditorApi = () => {
    if (!originalSetTheme || !originalCreate) return
    monaco.editor.setTheme = originalSetTheme
    monaco.editor.create = originalCreate
}

const applyFallbackMonacoTheme = (resolved: ResolvedTheme) => {
    monaco.editor.defineTheme(TAIDE_MONACO_THEME_NAME, buildMonacoThemeData(resolved))
    monaco.editor.setTheme(TAIDE_MONACO_THEME_NAME)
}

const attachShikiTokensProvider = async (resolved: ResolvedTheme) => {
    if (!highlighter) return
    await highlighter.loadTheme(buildShikiTheme(resolved))
    restoreOriginalMonacoEditorApi()
    shikiToMonaco(highlighter, monaco as ShikiMonacoNamespace)
}

/**
 * Drops `embeddedLangs` entries that are not among the grammars actually being loaded — shiki's
 * `Registry.loadLanguages` throws and aborts highlighter creation entirely if any embedded
 * language name is unresolved, so an unvalidated plugin manifest field must not reach it.
 */
const sanitizePluginGrammarEmbeddedLangs = (grammar: LanguageRegistration, loadedLanguageNames: ReadonlySet<string>): LanguageRegistration => {
    if (!grammar.embeddedLangs) return grammar
    const unresolved = grammar.embeddedLangs.filter((name) => !loadedLanguageNames.has(name))
    if (unresolved.length > 0) {
        console.warn(`[shiki] plugin grammar "${grammar.name}" declares unresolved embeddedLanguages, skipping: ${unresolved.join(', ')}`)
    }
    return { ...grammar, embeddedLangs: grammar.embeddedLangs.filter((name) => loadedLanguageNames.has(name)) }
}

const createConfiguredHighlighter = async (pluginGrammars: LanguageRegistration[]) => {
    const taideGrammars = await loadAllTaideGrammars()
    const loadedLanguageNames = new Set([...taideGrammars, ...pluginGrammars].map((grammar) => grammar.name))
    const safePluginGrammars = pluginGrammars.map((grammar) => sanitizePluginGrammarEmbeddedLangs(grammar, loadedLanguageNames))
    return createHighlighterCore({
        themes: [lastResolvedTheme ? buildShikiTheme(lastResolvedTheme) : PLACEHOLDER_SHIKI_THEME],
        langs: [...taideGrammars, ...safePluginGrammars],
        engine: createJavaScriptRegexEngine(),
    })
}

/**
 * Builds a fresh highlighter and installs it as the shared `highlighter`, reapplying
 * `lastResolvedTheme` through it. Failures (e.g. a malformed plugin grammar) are caught and
 * logged rather than left as a rejected promise — `highlighter` stays `null` and the editor keeps
 * using the pre-shiki fallback theme (`applyFallbackMonacoTheme`) until a retry.
 */
const buildAndInstallHighlighter = async (pluginGrammars: LanguageRegistration[]) => {
    try {
        captureOriginalMonacoEditorApi()
        highlighter = await createConfiguredHighlighter(pluginGrammars)
        if (lastResolvedTheme) await attachShikiTokensProvider(lastResolvedTheme)
    } catch (error) {
        console.error('[shiki] failed to initialize highlighter, keeping fallback monaco theming', error)
        highlighter = null
    }
}

/**
 * Loads all TAIDE + plugin grammars and creates the shared shiki highlighter. Idempotent — a
 * highlighter is only ever built once: a call while one already exists resolves immediately, and a
 * call while a build is still in flight (whether from an earlier `initShiki` or a `reinitShiki`)
 * queues behind it (`runExclusive`, F6#19) and then finds `highlighter` already set, so it never
 * redoes the (expensive) grammar-load/highlighter-create work.
 */
export const initShiki = (pluginGrammars: LanguageRegistration[]): Promise<void> => {
    if (highlighter) return Promise.resolve()
    return runExclusive(() => (highlighter ? Promise.resolve() : buildAndInstallHighlighter(pluginGrammars)))
}

/**
 * Actually applies `resolved` as the editor theme — the pre-shiki `defineTheme`/`setTheme` fallback
 * if `initShiki` hasn't finished yet (or was never called), otherwise the shiki path. Queued behind
 * `runExclusive` (see {@link operationQueue}) so it can never interleave with an in-flight
 * `initShiki`/`reinitShiki`.
 */
const applyResolvedShikiThemeExclusive = (resolved: ResolvedTheme) =>
    runExclusive(async () => {
        if (!highlighter) {
            applyFallbackMonacoTheme(resolved)
            return
        }
        await attachShikiTokensProvider(resolved)
    })

/**
 * Re-applies whatever `lastResolvedTheme` currently is — read at call time, not captured at
 * schedule time, so a trailing (debounced) run always reflects the latest theme even though it was
 * triggered by an earlier call (see {@link shikiThemeReapplyDebouncer}). Errors are caught and
 * logged here, not left to reject: a trailing run has no caller left waiting on its promise to
 * `.catch()`.
 */
const reapplyLastResolvedShikiTheme = () => {
    if (!lastResolvedTheme) return undefined
    return applyResolvedShikiThemeExclusive(lastResolvedTheme).catch((error: unknown) => console.error('[shiki] failed to apply theme', error))
}

/**
 * Leading+trailing debounced at {@link SHIKI_THEME_REAPPLY_DEBOUNCE_MS} (contract d-45 §0, §1#3): a
 * call that arrives while this debouncer is idle still applies immediately, while a flood of calls
 * (a theme editor color-picker drag re-runs `ThemeProvider`'s effect, hence `applyShikiTheme`, on
 * every rAF-coalesced preview frame) collapses to one re-apply after the flood actually stops instead
 * of one full monaco retokenize per frame. "Idle" matters (contract d-45 F-06): a boot or a manual
 * theme switch is only instant when it lands more than {@link SHIKI_THEME_REAPPLY_DEBOUNCE_MS} after
 * the previous apply — one that lands within that window (e.g. switching themes right as a drag's
 * last preview frame is still cooling down) is itself folded into the trailing run and so can lag the
 * (undebounced) CSS variables by up to that same window. The final state is always correct either way
 * (this always reads `lastResolvedTheme` at run time, never a stale capture) — only the sync between
 * CSS and monaco tokens is briefly out of step.
 */
const shikiThemeReapplyDebouncer = createLeadingTrailingDebouncer(reapplyLastResolvedShikiTheme, SHIKI_THEME_REAPPLY_DEBOUNCE_MS)

/**
 * Records `resolved` as the theme to apply and debounces the actual apply through
 * {@link shikiThemeReapplyDebouncer}. `lastResolvedTheme` is updated synchronously on every call
 * (never debounced) so `initShiki`/`reinitShiki` — which reapply `lastResolvedTheme` once their own
 * highlighter build finishes — always pick up the most recent theme regardless of where the
 * debounce window currently stands.
 *
 * Intentionally synchronous, not `async` (contract d-45 F-05): a call folded into the trailing edge
 * does not actually reapply until up to {@link SHIKI_THEME_REAPPLY_DEBOUNCE_MS} later, so an `async`
 * signature that resolved right away on every call would misrepresent "accepted" as "applied".
 * Completion is not observable from the caller at all — {@link reapplyLastResolvedShikiTheme} already
 * catches and logs its own failures, since a trailing run has no caller left waiting to `.catch()` it.
 */
export const applyShikiTheme = (resolved: ResolvedTheme) => {
    lastResolvedTheme = resolved
    shikiThemeReapplyDebouncer.trigger()
}

/**
 * Recreates the highlighter with a fresh grammar set (e.g. after plugins reload), reapplying
 * `lastResolvedTheme`. Queued behind `runExclusive` (F6#19) the same as `initShiki`, so this never
 * runs concurrently with another `initShiki`/`reinitShiki` call — the stale highlighter is disposed
 * only after the new one has fully taken over (monaco's tokens provider and `setTheme`/`create`
 * patches repointed to it) — disposing it up front would leave those still bound to the old
 * instance for the duration of the rebuild, and any retokenize/theme-change during that window
 * would call into a disposed highlighter.
 */
export const reinitShiki = (pluginGrammars: LanguageRegistration[]): Promise<void> =>
    runExclusive(async () => {
        const staleHighlighter = highlighter
        highlighter = null
        await buildAndInstallHighlighter(pluginGrammars)
        staleHighlighter?.dispose()
    })
