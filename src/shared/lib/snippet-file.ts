import type { SnippetStringOrList } from '@shared/api/bindings'

export const SNIPPET_LANGUAGE_FILE_EXTENSION = '.json'
export const SNIPPET_GLOBAL_FILE_EXTENSION = '.code-snippets'

/**
 * A snippet file is either a per-language `<languageId>.json` or a global `*.code-snippets` file
 * (VS Code's own naming split — contract §3.3). Shared by `shared/lib/snippet-completion.ts`'s
 * completion provider and `widgets/snippet-editor/snippet-draft.ts`'s editor state so the two
 * predicates can never drift apart.
 */
export const isGlobalSnippetFileName = (fileName: string) => fileName.endsWith(SNIPPET_GLOBAL_FILE_EXTENSION)

/**
 * VS Code's own semantics for a `string | string[]` snippet field: an array is multiple *lines*,
 * joined with `\n` (`body`, and `description` when hand-authored as an array) — never a
 * comma-separated list. Comma-separated multi-value editing (`prefix`'s multiple triggers) is a
 * TAIDE editor-UI convention layered on top of this, kept local to
 * `widgets/snippet-editor/snippet-draft.ts` rather than folded in here.
 */
export const stringOrListToLines = (value: SnippetStringOrList) => (Array.isArray(value) ? value.join('\n') : value)
