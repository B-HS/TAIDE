import { createHighlighterCore, type HighlighterCore, type LanguageRegistration, type ThemeRegistrationAny } from '@shikijs/core'
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript'
import { shikiToMonaco } from '@shikijs/monaco'
import type { ResolvedTheme } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { buildMonacoThemeData, TAIDE_MONACO_THEME_NAME } from '@shared/lib/monaco/theme'
import { buildShikiTheme } from '@shared/lib/shiki/build-shiki-theme'
import { loadAllTaideGrammars } from '@shared/lib/shiki/lang-map'

const PLACEHOLDER_SHIKI_THEME: ThemeRegistrationAny = { name: TAIDE_MONACO_THEME_NAME, type: 'dark', colors: {}, tokenColors: [] }

type ShikiMonacoNamespace = Parameters<typeof shikiToMonaco>[1]

type MonacoEditorSetTheme = typeof monaco.editor.setTheme
type MonacoEditorCreate = typeof monaco.editor.create

let highlighter: HighlighterCore | null = null
let lastResolvedTheme: ResolvedTheme | null = null
let originalSetTheme: MonacoEditorSetTheme | null = null
let originalCreate: MonacoEditorCreate | null = null

/**
 * Serializes every `initShiki`/`reinitShiki` body behind one FIFO chain (F6#19) — without this, two
 * concurrent calls (e.g. a plugin-reload-triggered `reinitShiki` racing an in-flight `initShiki`,
 * or two rapid plugin reloads) could each run `captureOriginalMonacoEditorApi` →
 * `createHighlighterCore` → `attachShikiTokensProvider`'s `restoreOriginalMonacoEditorApi` →
 * `shikiToMonaco` interleaved: `shikiToMonaco` monkeypatches `monaco.editor.setTheme`/`create`, and
 * two overlapping restore-then-repatch sequences can leave monaco's public API wrapped around a
 * highlighter that call B's own `reinitShiki` has since disposed, or leave call A's finished
 * highlighter's registration silently clobbered by call B's before call A ever disposed its own
 * stale one. Every operation queued here runs to completion before the next one starts, so
 * "restore original → repatch with the new highlighter → (reinit only) dispose the stale one" is
 * always atomic relative to any other call.
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
 * Applies `resolved` as the editor theme. Before `initShiki` finishes (or if it was never called
 * yet) this falls back to the pre-shiki `defineTheme`/`setTheme` path so the editor is never left
 * unthemed; once the highlighter is ready it is re-applied through the shiki path automatically
 * (`initShiki` re-runs this against `lastResolvedTheme` once the highlighter exists).
 */
export const applyShikiTheme = async (resolved: ResolvedTheme) => {
    lastResolvedTheme = resolved
    if (!highlighter) {
        applyFallbackMonacoTheme(resolved)
        return
    }
    await attachShikiTokensProvider(resolved)
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
