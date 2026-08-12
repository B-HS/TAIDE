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
let initPromise: Promise<void> | null = null
let lastResolvedTheme: ResolvedTheme | null = null
let originalSetTheme: MonacoEditorSetTheme | null = null
let originalCreate: MonacoEditorCreate | null = null

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

/**
 * Loads all TAIDE + plugin grammars and creates the shared shiki highlighter. Idempotent — a
 * second call while the first is still in flight returns the same promise. Failures (e.g. a
 * malformed plugin grammar) are caught and logged rather than left as a permanently-rejected
 * `initPromise` — `highlighter` stays `null` and the editor keeps using the pre-shiki fallback
 * theme (`applyFallbackMonacoTheme`) until a retry (e.g. "reload plugins" → `reinitShiki`).
 */
export const initShiki = async (pluginGrammars: LanguageRegistration[]) => {
    if (initPromise) return initPromise
    initPromise = (async () => {
        try {
            captureOriginalMonacoEditorApi()
            const taideGrammars = await loadAllTaideGrammars()
            const loadedLanguageNames = new Set([...taideGrammars, ...pluginGrammars].map((grammar) => grammar.name))
            const safePluginGrammars = pluginGrammars.map((grammar) => sanitizePluginGrammarEmbeddedLangs(grammar, loadedLanguageNames))
            highlighter = await createHighlighterCore({
                themes: [lastResolvedTheme ? buildShikiTheme(lastResolvedTheme) : PLACEHOLDER_SHIKI_THEME],
                langs: [...taideGrammars, ...safePluginGrammars],
                engine: createJavaScriptRegexEngine(),
            })
            if (lastResolvedTheme) await attachShikiTokensProvider(lastResolvedTheme)
        } catch (error) {
            console.error('[shiki] failed to initialize highlighter, keeping fallback monaco theming', error)
            highlighter = null
            initPromise = null
        }
    })()
    return initPromise
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
 * `lastResolvedTheme`. The stale highlighter is disposed only after the new one has fully taken
 * over (monaco's tokens provider and `setTheme`/`create` patches repointed to it) — disposing it
 * up front would leave those still bound to the old instance for the duration of the rebuild,
 * and any retokenize/theme-change during that window would call into a disposed highlighter.
 */
export const reinitShiki = async (pluginGrammars: LanguageRegistration[]) => {
    const staleHighlighter = highlighter
    highlighter = null
    initPromise = null
    await initShiki(pluginGrammars)
    staleHighlighter?.dispose()
}
