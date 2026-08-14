/**
 * Ambient shims for monaco-editor internal ESM modules that `command-relay.ts` deep-imports.
 *
 * `monaco-editor`'s own `package.json` `exports` map deliberately re-exposes every module under
 * `esm/vs/*` via a wildcard (`"./*": "./esm/vs/*.js"`), so these subpaths are a supported, if
 * untyped, part of the package's surface — TypeScript just has no declaration file for them
 * (only the top-level `editor.main.d.ts` is shipped). See
 * `docs/acknowledge/2026-08-14-monaco-command-service-deep-import.md` for why this reach-in
 * exists (no *public* monaco.d.ts API to invoke an already-registered command with more than one
 * positional argument — `editor.trigger` only forwards a single payload) and what to check on a
 * monaco upgrade. `ICommandService`/`StandaloneServices.get` is the same mechanism monaco's own
 * `registerCommandAlias` uses internally.
 */
declare module 'monaco-editor/platform/commands/common/commands' {
    export const ICommandService: unknown
}

declare module 'monaco-editor/editor/standalone/browser/standaloneServices' {
    export const StandaloneServices: {
        get: (identifier: unknown) => unknown
    }
}
