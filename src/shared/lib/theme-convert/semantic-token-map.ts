import type { SYNTAX_TOKENS } from '@shared/lib/theme-convert/ui-token-vocabulary'

/**
 * Maps every semantic token *type* string a supported LSP server's `legend.tokenTypes` can name
 * to one of TAIDE's 31 `SYNTAX_TOKENS`. Three vocabularies are merged into one flat lookup: LSP
 * 3.17's standard 23 types (`protocol.ts`'s `SEMANTIC_TOKEN_TYPES`), rust-analyzer's non-standard
 * `SupportedType` extensions (source-verified against `crates/rust-analyzer/src/lsp/semantic_tokens.rs`),
 * and clangd's non-standard `toSemanticTokenType` extensions (source-verified against
 * `clang-tools-extra/clangd/SemanticHighlighting.cpp`) — vtsls stays within the standard 23. A type
 * absent from this map is dropped by the adapter (`adapters/semantic-tokens.ts`, via
 * {@link lookupSemanticTokenTypeMapping}) rather than guessed at — silently losing a semantic color
 * is safe, painting the wrong one is not
 * (`docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md` §2-1's washout finding).
 *
 * Non-obvious picks: `struct`→class (no dedicated struct token), `typeParameter`/`builtinType`/
 * `typeAlias`/`union`/`generic`/`concept`→type, `enumMember`/`boolean`/`constParameter`/`const`/
 * `static`→constant (immutable, language-level values), `event`→property (LSP's own definition: "a
 * property or member that acts as an event"), `modifier`/`selfKeyword`/`selfTypeKeyword`/`label`→
 * keyword, `lifetime`→storage (a `storage.modifier`-shaped annotation, not a type),
 * `macro`/`macroBang`/`procMacro`→function (rust's own TextMate grammar already scopes macro
 * invocations as `entity.name.function.macro`), `formatSpecifier`/`escapeSequence`→regexp (both are
 * string-interior sub-highlights, same family as `string.regexp`), `invalidEscapeSequence`→invalid
 * (an error indicator, not a regular string-interior highlight), the `punctuation` sub-kinds
 * (brace/bracket/parenthesis/comma/colon/semicolon/dot/angle) plus the bare `punctuation` type
 * itself→punctuation, `attribute`/`attributeBracket`/`builtinAttribute`/`derive`/`deriveHelper`→
 * decorator, `character`→string, the operator sub-kinds (arithmetic/bitwise/comparison/logical/
 * negation)→operator, `toolModule`→namespace. `unresolvedReference` (rust-analyzer) and `unknown`
 * (clangd's own explicitly-"nonstandard" catch-all) are intentionally absent — dropped, not colored
 * as an error/guessed at (that is `diagnosticProvider`'s job for the former; the latter has no
 * confident target by clangd's own definition).
 */
export const SEMANTIC_TOKEN_TYPE_MAP: Record<string, (typeof SYNTAX_TOKENS)[number]> = {
    namespace: 'namespace',
    type: 'type',
    class: 'class',
    enum: 'enum',
    interface: 'interface',
    struct: 'class',
    typeParameter: 'type',
    parameter: 'parameter',
    variable: 'variable',
    property: 'property',
    enumMember: 'constant',
    event: 'property',
    function: 'function',
    method: 'method',
    macro: 'function',
    keyword: 'keyword',
    modifier: 'keyword',
    comment: 'comment',
    string: 'string',
    number: 'number',
    regexp: 'regexp',
    operator: 'operator',
    decorator: 'decorator',

    builtinType: 'type',
    typeAlias: 'type',
    union: 'type',
    generic: 'type',
    concept: 'type',
    selfKeyword: 'keyword',
    selfTypeKeyword: 'keyword',
    label: 'keyword',
    lifetime: 'storage',
    formatSpecifier: 'regexp',
    escapeSequence: 'regexp',
    invalidEscapeSequence: 'invalid',
    brace: 'punctuation',
    bracket: 'punctuation',
    parenthesis: 'punctuation',
    comma: 'punctuation',
    colon: 'punctuation',
    semicolon: 'punctuation',
    dot: 'punctuation',
    angle: 'punctuation',
    punctuation: 'punctuation',
    attribute: 'decorator',
    attributeBracket: 'decorator',
    builtinAttribute: 'decorator',
    derive: 'decorator',
    deriveHelper: 'decorator',
    boolean: 'constant',
    constParameter: 'constant',
    const: 'constant',
    static: 'constant',
    character: 'string',
    arithmetic: 'operator',
    bitwise: 'operator',
    comparison: 'operator',
    logical: 'operator',
    negation: 'operator',
    macroBang: 'function',
    procMacro: 'function',
    toolModule: 'namespace',
}

/**
 * Namespace `buildSemanticTokenThemeRules` (`build-shiki-theme.ts`) and `buildSemanticTokensLegendMapping`
 * (`adapters/semantic-tokens.ts`) share for every theme rule scope / monaco-facing legend token type
 * name they emit for a `SEMANTIC_TOKEN_TYPE_MAP` target — so neither ever exact-matches a real
 * TextMate scope. A bare `SYNTAX_TOKENS` name (e.g. `'variable'`) *is* a real TextMate scope many
 * bundled themes' own `tokenColors` already use verbatim; inserting a theme rule under that exact
 * scope string collides with — and, per monaco's token-theme trie (`resolveParsedTokenThemeRules`
 * sorts rules by token string then array index, and `ThemeTrieElement.insert`'s exact-match case
 * calls `acceptOverwrite`, so the later-index rule for an identical token string always wins) —
 * silently overwrites the theme's own rule for it, corrupting regular syntax colors alongside
 * semantic ones. No TextMate grammar emits a scope under this namespace, so a rule scoped here can
 * never collide with one.
 */
export const SEMANTIC_TOKEN_LEGEND_SCOPE_PREFIX = 'taideSemantic'

export const toSemanticTokenLegendScope = (token: (typeof SYNTAX_TOKENS)[number]) => `${SEMANTIC_TOKEN_LEGEND_SCOPE_PREFIX}.${token}`

/**
 * Looks up `typeName` in {@link SEMANTIC_TOKEN_TYPE_MAP} without walking the prototype chain — a
 * plain object literal inherits from `Object.prototype`, so an unguarded `SEMANTIC_TOKEN_TYPE_MAP[typeName]`
 * returns `constructor`/`toString`/`valueOf`/etc. instead of `undefined` for a server legend that
 * happens to declare a type with one of those names, which downstream would turn into an
 * out-of-range semantic token type index instead of the intended "drop" behavior.
 */
export const lookupSemanticTokenTypeMapping = (typeName: string): (typeof SYNTAX_TOKENS)[number] | undefined =>
    Object.hasOwn(SEMANTIC_TOKEN_TYPE_MAP, typeName) ? SEMANTIC_TOKEN_TYPE_MAP[typeName] : undefined
