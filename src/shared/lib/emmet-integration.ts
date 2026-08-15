import { emmetCSS, emmetHTML, emmetJSX } from 'emmet-monaco-es'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { TaideLanguageId } from '@shared/lib/shiki/lang-map'

/**
 * `tokenizer: 'standard'` is required whenever the editor's tokens come from a non-Monarch engine
 * — TAIDE always drives tokenization via `@shikijs/monaco`'s `setTokensProvider`
 * (`shared/lib/shiki/shiki-monaco.ts`), never monaco's built-in Monarch tokenizer. The library's
 * default `'monarch'` tokenizer reflects into monaco-internal Monarch state that a shiki-tokenized
 * model never populates, so abbreviation detection silently never fires without this option
 * (contract §2-8, confirmed against the library's own README).
 */
const EMMET_OPTIONS = { tokenizer: 'standard' } as const

/** `heex` reuses the shiki `html` grammar (`shiki/lang-map.ts`), so it belongs to the HTML family here too. */
const HTML_LANGUAGE_IDS: TaideLanguageId[] = ['html', 'heex']
const CSS_LANGUAGE_IDS: TaideLanguageId[] = ['css', 'scss']
const JSX_LANGUAGE_IDS: TaideLanguageId[] = ['javascriptreact', 'typescriptreact']

/**
 * Registers Emmet abbreviation expansion for the HTML/CSS/JSX language families, matching them
 * against `TAIDE_LANGUAGE_IDS`'s actual names (contract §3.4) rather than the library's own
 * English-language defaults (`['html']`/`['css']`/`['javascript']`). Returns one combined dispose
 * so the `emmet_enabled` settings toggle (wired in `app/`) can tear down and re-register as a unit.
 */
export const enableEmmet = (monaco: Monaco) => {
    const disposers = [
        emmetHTML(monaco, HTML_LANGUAGE_IDS, EMMET_OPTIONS),
        emmetCSS(monaco, CSS_LANGUAGE_IDS, EMMET_OPTIONS),
        emmetJSX(monaco, JSX_LANGUAGE_IDS, EMMET_OPTIONS),
    ]
    return () => disposers.forEach((dispose) => dispose())
}
