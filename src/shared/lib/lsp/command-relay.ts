import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location, LocationLink, LspPosition } from '@shared/lib/lsp/protocol'
import { lspPositionToMonaco, lspRangeToMonaco } from '@shared/lib/lsp/position'

type Disposable = { dispose: () => void }

const NOOP_DISPOSABLE: Disposable = { dispose: () => {} }

/** A command reference as it appears on an LSP `CodeAction`/`CodeLens` (`Command`). */
export type LspCommand = { title: string; command: string; arguments?: unknown[] }

type MonacoCommandService = { executeCommand: (id: string, ...args: unknown[]) => Promise<unknown> }

/**
 * Deep-imported dynamically (not as a static top-level import) so merely importing this module
 * never evaluates monaco's browser services chain — that chain reads `window` at module-eval time
 * (`base/browser/window.js`), which throws outside a real DOM (including under `bun:test`). A
 * static import would make every consumer of this file's *other*, DOM-free exports (the pure
 * arg-conversion handlers below) fail to even load in that environment. See `monaco-internal.d.ts`
 * for why this deep-imports monaco's internal `ICommandService` at all: there is no public
 * monaco.d.ts API to invoke an already-registered command with more than one positional argument.
 */
const loadMonacoCommandService = async (): Promise<MonacoCommandService> => {
    const [{ ICommandService }, { StandaloneServices }] = await Promise.all([
        import('monaco-editor/platform/commands/common/commands'),
        import('monaco-editor/editor/standalone/browser/standaloneServices'),
    ])
    return StandaloneServices.get(ICommandService) as MonacoCommandService
}

/** Signature of {@link executeMonacoCommand}, factored out so tests can inject a fake and never touch real monaco DI. */
export type MonacoCommandExecutor = (id: string, ...args: unknown[]) => Promise<unknown>

/**
 * Executes an already-registered monaco command by id with positional arguments. There is no
 * *public* monaco.d.ts API for this (`editor.trigger(source, id, payload)` only forwards a single
 * payload object, not positional args). Used both to invoke monaco's built-in commands (e.g.
 * `editor.action.goToLocations`) and, indirectly, to run whatever a `registerCommand` call below
 * wires up for a given id.
 *
 * Unverified by unit test the same way `lsp-session-registry.ts` is (both need a real DOM) — the
 * argument-conversion logic that matters is covered instead via
 * {@link createShowReferencesHandler}/{@link createGotoLocationHandler} with an injected
 * `MonacoCommandExecutor` fake.
 */
export const executeMonacoCommand: MonacoCommandExecutor = async (id, ...args) => (await loadMonacoCommandService()).executeCommand(id, ...args)

/** Runs an LSP `Command` (from a resolved `CodeAction.command` or `CodeLens.command`) as a monaco command. */
export const executeLspCommand = (command: LspCommand) => executeMonacoCommand(command.command, ...(command.arguments ?? []))

/**
 * Registers every id a session's `executeCommandProvider.commands` advertises, relaying invocation
 * to `workspace/executeCommand` on that session's client — this is how gopls-style code actions
 * (which return a bare `Command` the server executes itself, then pushes a `workspace/applyEdit`
 * request back) and CodeLens click handlers actually run. Re-registering the same id (e.g. two
 * sessions of the same server) is safe: monaco's `CommandsRegistry` keeps the most-recently
 * registered handler active and restores the previous one once this disposable's registration is
 * torn down, so "last write wins" falls out of monaco's own registry rather than needing bookkeeping
 * here. Caller (Phase D bootstrap / session lifecycle) owns *when* this is called and disposed.
 */
export const registerSessionExecuteCommands = (monaco: Monaco, client: LspClient, commands: readonly string[] | undefined): Disposable => {
    if (!commands?.length) return NOOP_DISPOSABLE

    /**
     * Returns (does not swallow) the `workspace/executeCommand` request promise — monaco's own
     * command service awaits whatever a registered command handler returns (confirmed against its
     * source), and on-save (`editor-pane.tsx`'s `applyCodeActionOrCommand`) relies on that await to
     * sequence "edit applied, now run its command" deterministically. A fire-and-forget handler
     * here would let both proceed before the server's own edit (e.g. a gopls-style bare `Command`
     * that pushes a `workspace/applyEdit` back) has actually landed.
     */
    const disposables = commands.map((commandId) =>
        monaco.editor.registerCommand(commandId, (_accessor: unknown, ...args: unknown[]) =>
            client.request('workspace/executeCommand', { command: commandId, arguments: args }),
        ),
    )

    return { dispose: () => disposables.forEach((disposable) => disposable.dispose()) }
}

/** Built-in monaco command that `editor.action.showReferences` (see below) delegates to for the actual peek/goto. */
const MONACO_GO_TO_LOCATIONS_COMMAND_ID = 'editor.action.goToLocations'

/**
 * VS Code's `editor.action.showReferences(uri, position, locations)` id. rust-analyzer and
 * typescript-language-server both send CodeLens commands with this exact id and LSP-shaped
 * (JSON) arguments — monaco itself aliases this id to `editor.action.peekLocations`, but that
 * alias asserts its first argument is a real `monaco.Uri` instance and throws on a plain string,
 * so it cannot consume LSP command arguments unmodified. Registering our own handler here (monaco
 * lets the most recent registration for an id win) intercepts before that assertion and converts.
 */
export const MONACO_SHOW_REFERENCES_COMMAND_ID = 'editor.action.showReferences'

/** rust-analyzer's client-only alias for the same `(uri, position, locations)` shape as {@link MONACO_SHOW_REFERENCES_COMMAND_ID}. */
export const RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID = 'rust-analyzer.showReferences'

/** rust-analyzer's client-only "open this single location" command — `arguments: [Location | LocationLink]`. */
export const RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID = 'rust-analyzer.gotoLocation'

const isLspPosition = (value: unknown): value is LspPosition =>
    typeof value === 'object' &&
    value !== null &&
    typeof (value as LspPosition).line === 'number' &&
    typeof (value as LspPosition).character === 'number'

const isLspLocationOrLocationLink = (value: unknown): value is Location | LocationLink =>
    typeof value === 'object' &&
    value !== null &&
    (typeof (value as Location).uri === 'string' || typeof (value as LocationLink).targetUri === 'string')

/**
 * A `LocationLink`'s `targetRange` spans the whole declaration (doc comments, attributes and all);
 * `targetSelectionRange` is the precise identifier span meant to receive the cursor/highlight
 * (LSP 3.17 `LocationLink`). Both are preserved here — dropping `targetSelectionRange` (as this
 * used to) makes monaco fall back to `range` (=`targetRange`) for both, landing the cursor on the
 * declaration's doc comment instead of the symbol itself.
 */
const toMonacoLocationArg = (monaco: Monaco, item: Location | LocationLink) =>
    'targetUri' in item
        ? {
              uri: monaco.Uri.parse(item.targetUri),
              range: lspRangeToMonaco(item.targetRange),
              targetSelectionRange: lspRangeToMonaco(item.targetSelectionRange),
              ...(item.originSelectionRange ? { originSelectionRange: lspRangeToMonaco(item.originSelectionRange) } : {}),
          }
        : { uri: monaco.Uri.parse(item.uri), range: lspRangeToMonaco(item.range) }

/**
 * Builds the `editor.action.showReferences` / `rust-analyzer.showReferences` handler: opens
 * monaco's built-in peek widget over `locations`, anchored at `position` in `uri`. `multiple:
 * 'peek'` + `openInPeek: true` on `editor.action.goToLocations` reproduces exactly what monaco's
 * own `peekLocations` → `goToLocations` chain does for a single reference, just with LSP-shaped
 * args converted first. `execute` defaults to the real {@link executeMonacoCommand} and is only
 * overridden by tests (which cannot construct a real `MonacoCommandExecutor`, see its doc).
 */
export const createShowReferencesHandler =
    (monaco: Monaco, execute: MonacoCommandExecutor = executeMonacoCommand) =>
    (_accessor: unknown, uri: unknown, position: unknown, locations: unknown) => {
        if (typeof uri !== 'string' || !isLspPosition(position) || !Array.isArray(locations)) return undefined
        const monacoLocations = locations.filter(isLspLocationOrLocationLink).map((location) => toMonacoLocationArg(monaco, location))
        return execute(
            MONACO_GO_TO_LOCATIONS_COMMAND_ID,
            monaco.Uri.parse(uri),
            lspPositionToMonaco(position),
            monacoLocations,
            'peek',
            undefined,
            true,
        )
    }

/** Builds the `rust-analyzer.gotoLocation` handler: navigates directly to a single `Location`/`LocationLink`, no peek. */
export const createGotoLocationHandler =
    (monaco: Monaco, execute: MonacoCommandExecutor = executeMonacoCommand) =>
    (_accessor: unknown, location: unknown) => {
        if (!isLspLocationOrLocationLink(location)) return undefined
        const target = toMonacoLocationArg(monaco, location)
        const focusRange = 'targetSelectionRange' in target ? target.targetSelectionRange : target.range
        const position = { lineNumber: focusRange.startLineNumber, column: focusRange.startColumn }
        return execute(MONACO_GO_TO_LOCATIONS_COMMAND_ID, target.uri, position, [target], 'goto')
    }

/**
 * Registers the fixed set of client-only navigation commands LSP servers reference by id from
 * CodeLens/CodeAction responses (not part of any session's `executeCommandProvider`, so
 * {@link registerSessionExecuteCommands} never covers them). Global and idempotent-per-call —
 * intended to be called once at app bootstrap (Phase D), independent of any single LSP session.
 */
export const registerLspClientNavigationCommands = (monaco: Monaco): Disposable => {
    const disposables = [
        monaco.editor.registerCommand(MONACO_SHOW_REFERENCES_COMMAND_ID, createShowReferencesHandler(monaco)),
        monaco.editor.registerCommand(RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID, createShowReferencesHandler(monaco)),
        monaco.editor.registerCommand(RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID, createGotoLocationHandler(monaco)),
    ]
    return { dispose: () => disposables.forEach((disposable) => disposable.dispose()) }
}
