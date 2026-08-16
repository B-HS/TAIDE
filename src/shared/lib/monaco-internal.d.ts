/**
 * Ambient shim for the monaco-editor internal ESM module that `keymap-when.ts` deep-imports.
 *
 * `monaco-editor`'s own `package.json` `exports` map re-exposes every module under `esm/vs/*` via
 * a wildcard (`"./*": "./esm/vs/*.js"`), so this subpath is a supported, if untyped, part of the
 * package's surface — TypeScript just has no declaration file for it (only the top-level
 * `editor.main.d.ts` is shipped). See `docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md`
 * for why this reach-in exists (there is no *public* monaco.d.ts API to parse/evaluate a `when`
 * clause string outside a real `IContextKeyService` instance) and what to check on a monaco
 * upgrade. This follows the same deep-import precedent as
 * `src/shared/lib/lsp/monaco-internal.d.ts` (`command-relay.ts`'s `ICommandService`), kept as a
 * separate file here because it shims a different consumer domain (keymap `when` evaluation, not
 * LSP command relay).
 */
declare module 'monaco-editor/platform/contextkey/common/contextkey' {
    /** The parsed form of a `when` clause string — see `ContextKeyExpr.deserialize`. */
    export type ContextKeyExpression = {
        evaluate: (context: { getValue: (key: string) => unknown }) => boolean
        /** Every context key this expression references (e.g. `'a && !b'` → `['a', 'b']`) — used to validate `when` strings only name whitelisted keys. */
        keys: () => string[]
    }

    export const ContextKeyExpr: {
        /** Parses a `when` clause (e.g. `"a && !b"`) into an evaluable expression, or `undefined` if the string is empty or malformed. */
        deserialize: (serialized: string) => ContextKeyExpression | undefined
    }
}
