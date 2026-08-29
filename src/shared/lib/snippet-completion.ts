import type { SnippetEntry, SnippetFile, SnippetStringOrList } from '@shared/api/bindings'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'
import { SNIPPET_LANGUAGE_FILE_EXTENSION, isGlobalSnippetFileName, stringOrListToLines } from '@shared/lib/snippet-file'

/**
 * `shared` cannot import `entities/snippet` (fsd.md §2 — no `shared` → `entities` reference), so
 * the caller (wired in `app/` at bootstrap, per the contract §3.3) injects a synchronous getter
 * over whatever it already holds cached (the `QUERY_KEY.SNIPPET.LIST` TanStack Query data) instead
 * of this module reaching for the IPC/query layer itself. Mirrors `workspace-edit-applier.ts`'s
 * same-layer-only rule for `shared/lib/lsp`.
 */
export type SnippetCompletionDeps = {
    getSnippetFiles: () => readonly SnippetFile[]
}

export type SnippetCompletionCandidate = {
    name: string
    prefix: string
    body: string
    description?: string
}

/** `description` is optional, unlike `body`/`prefix` — `null`/`undefined` collapses to `undefined` (monaco's completion `documentation` renders nothing), everything else goes through the same line-join as `body`. */
const toOptionalText = (value: SnippetStringOrList | null | undefined) =>
    value === null || value === undefined ? undefined : stringOrListToLines(value)

const toPrefixes = (value: SnippetStringOrList) => (Array.isArray(value) ? value : [value])

const parseScopeList = (scope: string | null | undefined) =>
    (scope ?? '')
        .split(',')
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0)

/**
 * A `.code-snippets` entry with no `scope` applies to every language (VS Code's own default);
 * one with a scope only applies where its comma-separated languageId list includes `languageId`.
 */
const matchesScope = (scope: string | null | undefined, languageId: string) => {
    const scopeList = parseScopeList(scope)
    return scopeList.length === 0 || scopeList.includes(languageId)
}

/**
 * A snippet file applies to `languageId` when it's that language's own `<languageId>.json` file,
 * or any global `*.code-snippets` file (whose individual entries are then further filtered by
 * `matchesScope`). Mirrors the Rust-side `<languageId>.json` / `*.code-snippets` split
 * (`domain/snippet/service.rs`'s `has_recognized_snippet_extension`) without importing it —
 * `shared` cannot depend on the Rust layer, so the predicate is necessarily re-expressed here; see
 * the contract §3.3 for why no separate language field is carried on `SnippetFile` itself.
 */
const fileAppliesToLanguage = (fileName: string, languageId: string) =>
    isGlobalSnippetFileName(fileName) || fileName === `${languageId}${SNIPPET_LANGUAGE_FILE_EXTENSION}`

export const collectSnippetCompletionCandidates = (files: readonly SnippetFile[], languageId: string): SnippetCompletionCandidate[] =>
    files
        .filter((file) => fileAppliesToLanguage(file.fileName, languageId))
        .flatMap((file) =>
            (Object.entries(file.snippets) as [string, SnippetEntry][])
                .filter(([, entry]) => !isGlobalSnippetFileName(file.fileName) || matchesScope(entry.scope, languageId))
                .flatMap(([name, entry]) =>
                    toPrefixes(entry.prefix).map((prefix) => ({
                        name,
                        prefix,
                        body: stringOrListToLines(entry.body),
                        description: toOptionalText(entry.description),
                    })),
                ),
        )

type SnippetCompletionInstallation = {
    monaco: Monaco
    deps: SnippetCompletionDeps
    disposablesByLanguageId: Map<string, { dispose: () => void }>
}

/**
 * The single {@link registerSnippetCompletions} installation, plus every plugin-contributed language
 * id requested so far. Module state (rather than a handle threaded through callers) because the two
 * halves are wired in different layers and in an order neither controls: `app/bootstrap-snippets.ts`
 * installs at import time, while plugin languages only become known once `PLUGIN.LIST` resolves —
 * and `shared` cannot reach either of them. Requested ids are kept, not drained, so an installation
 * created after them still covers them.
 */
let installation: SnippetCompletionInstallation | null = null
const pluginLanguageIds = new Set<string>()

const registerSnippetCompletionsForLanguage = (languageId: string) => {
    if (!installation || installation.disposablesByLanguageId.has(languageId)) return
    const { monaco, deps, disposablesByLanguageId } = installation

    disposablesByLanguageId.set(
        languageId,
        monaco.languages.registerCompletionItemProvider(languageId, {
            provideCompletionItems: (model, position) => {
                const word = model.getWordUntilPosition(position)
                const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn)
                const suggestions = collectSnippetCompletionCandidates(deps.getSnippetFiles(), languageId).map((candidate) => ({
                    label: candidate.prefix,
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: candidate.body,
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: candidate.description,
                    detail: candidate.name,
                    range,
                }))
                return { suggestions }
            },
        }),
    )
}

/**
 * Registers one `languages.CompletionItemProvider` per `TAIDE_LANGUAGE_IDS` entry (plus every
 * plugin language {@link registerSnippetCompletionsForLanguages} has been told about) — never a
 * single `'*'` selector, which VS Code/monaco scores into a low-priority suggestion group that a
 * language with an LSP attached permanently outranks (contract §3.3, re-verified against monaco's
 * `suggest.js`). Meant to be called once at app bootstrap; the returned disposable tears down all
 * of them together.
 */
export const registerSnippetCompletions = (monaco: Monaco, deps: SnippetCompletionDeps) => {
    installation = { monaco, deps, disposablesByLanguageId: new Map() }
    const registered = installation.disposablesByLanguageId
    ;[...TAIDE_LANGUAGE_IDS, ...pluginLanguageIds].forEach(registerSnippetCompletionsForLanguage)

    return {
        dispose: () => {
            registered.forEach((disposable) => disposable.dispose())
            registered.clear()
            if (installation?.disposablesByLanguageId === registered) installation = null
        },
    }
}

/**
 * Extends snippet completions to plugin-contributed language ids. Providers are registered per exact
 * language id (see {@link registerSnippetCompletions}), and the bootstrap set is the static
 * `TAIDE_LANGUAGE_IDS` list, so a language a plugin adds had no provider at all: neither its own
 * `<languageId>.json` snippet file nor a global `.code-snippets` entry scoped to it could ever fire
 * (audit §4-B D6). Called from `registerPluginLanguages` so both of that function's callers (the
 * boot-time plugin list and every later install/uninstall/reload) are covered by construction.
 * Idempotent per id, and safe to call before the bootstrap installation exists.
 */
export const registerSnippetCompletionsForLanguages = (languageIds: readonly string[]) => {
    languageIds.forEach((languageId) => {
        pluginLanguageIds.add(languageId)
        registerSnippetCompletionsForLanguage(languageId)
    })
}
