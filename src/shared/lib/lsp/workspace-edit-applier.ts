import { commands } from '@shared/api/bindings'
import { IpcError, unwrapResult } from '@shared/api/unwrap-result'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { markModelDirtyExternally } from '@shared/lib/lsp/model-dirty-tracker'
import { isPeekPreloadedModel } from '@shared/lib/lsp/peek-model-preload'
import type {
    CreateFile,
    DeleteFile,
    DocumentChangeOperation,
    OptionalVersionedTextDocumentIdentifier,
    RenameFile,
    TextDocumentEdit,
    TextEdit,
    WorkspaceEdit,
} from '@shared/lib/lsp/protocol'
import { lspRangeToMonaco } from '@shared/lib/lsp/position'

/** Mirrors LSP's `ApplyWorkspaceEditResult` wire shape so handlers can return this value as-is. */
export type WorkspaceEditApplyResult = { applied: boolean; failureReason?: string }

/**
 * Thin `unwrapResult(commands.fileXxx(...))` wrappers, kept local instead of importing
 * `@entities/file/file.ipc` — this module lives in `shared/lib/lsp`, and FSD forbids `shared`
 * importing `entities` (fsd.md §2; every other `shared/lib/lsp` adapter avoids it the same way).
 * `@shared/api/bindings`/`@shared/api/unwrap-result` are the shared-layer primitives the entities
 * wrapper itself is built from, so this is a same-layer call, not a new IPC surface.
 */
const openFile = (path: string) => unwrapResult(commands.fileOpen(path))
const saveFile = (input: { path: string; content: string }) => unwrapResult(commands.fileSave(input.path, input.content))
const createEntry = (input: { path: string; isDir: boolean }) => unwrapResult(commands.fileCreate(input.path, input.isDir))
const renameEntry = (input: { from: string; to: string }) => unwrapResult(commands.fileRename(input.from, input.to))
const deleteEntry = (path: string) => unwrapResult(commands.fileDelete(path))

/**
 * File-IPC side of the applier, factored out for dependency injection (`common.md`/`backend.md`
 * Factory-DI spirit applied on the frontend): tests substitute fakes here instead of touching the
 * real IPC bridge. Whether a uri is an already-open monaco model is read straight off the `monaco`
 * instance passed in (`monaco.editor.getModel`), not injected here — every caller of this module
 * already carries a real `Monaco` (adapters receive it as a parameter, never import the
 * `@shared/lib/monaco/setup` singleton directly), so a fake `Monaco` in tests covers it too.
 */
export type WorkspaceEditApplierDeps = {
    openFile: typeof openFile
    saveFile: typeof saveFile
    createEntry: typeof createEntry
    renameEntry: typeof renameEntry
    deleteEntry: typeof deleteEntry
}

const defaultDeps: WorkspaceEditApplierDeps = { openFile, saveFile, createEntry, renameEntry, deleteEntry }

const APPLIED: WorkspaceEditApplyResult = { applied: true }

const failure = (error: unknown): WorkspaceEditApplyResult => ({
    applied: false,
    failureReason: error instanceof Error ? error.message : String(error),
})

const uriToPath = (monaco: Monaco, uri: string) => monaco.Uri.parse(uri).fsPath

const isNotFoundError = (error: unknown) => error instanceof IpcError && error.code === 'NotFound'

const isAlreadyExistsError = (error: unknown) => error instanceof IpcError && error.code === 'InvalidArgument'

/**
 * Extra, optional constraints an `applyWorkspaceEdit` caller can impose on top of the base
 * apply logic. Both are opt-in (undefined = unrestricted) because only the server-initiated
 * `workspace/applyEdit` push (`workspace-edit-apply-handler.ts`, registered per LSP session with
 * that session's own root) currently supplies them — interactive rename/code-action requests act
 * on locations the same server already returned for the file the user is looking at.
 */
export type WorkspaceEditApplyOptions = {
    /** When set, every operation's target path(s) must resolve under this root or the whole edit is rejected. */
    allowedRoot?: string
    /** When set, a `TextDocumentEdit` whose `textDocument.version` doesn't match the client's tracked version for that uri is rejected instead of applied against stale offsets. */
    getDocumentVersion?: (uri: string) => number | undefined
}

const OUTSIDE_ROOT_FAILURE: WorkspaceEditApplyResult = { applied: false, failureReason: 'edit rejected: outside workspace root' }

const isWithinRoot = (path: string, root: string) => path === root || path.startsWith(`${root}/`)

const assertPathsWithinRoot = (monaco: Monaco, allowedRoot: string | undefined, uris: readonly string[]): WorkspaceEditApplyResult | null => {
    if (allowedRoot === undefined) return null
    const outside = uris.some((uri) => !isWithinRoot(uriToPath(monaco, uri), allowedRoot))
    return outside ? OUTSIDE_ROOT_FAILURE : null
}

const assertVersionCurrent = (
    textDocument: OptionalVersionedTextDocumentIdentifier,
    getDocumentVersion: WorkspaceEditApplyOptions['getDocumentVersion'],
): WorkspaceEditApplyResult | null => {
    if (!getDocumentVersion || textDocument.version === null) return null
    const current = getDocumentVersion(textDocument.uri)
    if (current === undefined || current === textDocument.version) return null
    return { applied: false, failureReason: `stale document version: ${textDocument.uri}` }
}

/**
 * `openFile` is the only existing IPC that can observe whether a path exists (it maps a missing
 * path to `AppError::NotFound`). Reused here as an existence probe for rename/delete resource
 * operations rather than adding a dedicated `file_exists` command — any error other than
 * `NotFound` (a directory, a refused binary, a permission error, ...) is treated as "exists"
 * since the path is reachable on disk either way.
 */
const pathExists = async (deps: WorkspaceEditApplierDeps, path: string) => {
    try {
        await deps.openFile(path)
        return true
    } catch (error) {
        return !isNotFoundError(error)
    }
}

const LINE_TERMINATOR_PATTERN = /\r\n|\r|\n/g

/** Byte-for-character offset of the start of each line, indexed by (0-based) line number. */
const buildLineStartOffsets = (content: string) => {
    const offsets = [0]
    for (const match of content.matchAll(LINE_TERMINATOR_PATTERN)) offsets.push((match.index ?? 0) + match[0].length)
    return offsets
}

const toOffset = (lineStartOffsets: number[], contentLength: number, position: { line: number; character: number }) =>
    (lineStartOffsets[position.line] ?? contentLength) + position.character

/**
 * Applies `edits` to a raw string outside of monaco (a file with no open model). Edits are sorted
 * by descending start offset and spliced back-to-front so an earlier-in-the-array edit's range
 * never gets invalidated by a later one shifting the string — the offset-safety monaco's own
 * `pushEditOperations` provides for free on an open model.
 */
export const applyTextEditsToContent = (content: string, edits: readonly TextEdit[]) => {
    const lineStartOffsets = buildLineStartOffsets(content)
    const withOffsets = edits
        .map((edit) => ({
            edit,
            start: toOffset(lineStartOffsets, content.length, edit.range.start),
            end: toOffset(lineStartOffsets, content.length, edit.range.end),
        }))
        .toSorted((a, b) => b.start - a.start)

    return withOffsets.reduce((current, { edit, start, end }) => current.slice(0, start) + edit.newText + current.slice(end), content)
}

/**
 * `monaco.editor.getModel(uri)` alone cannot distinguish a genuinely open, saved-through document
 * from a peek-preview model `peek-model-preload.ts` created ahead of time at the same real file
 * uri (so monaco's Peek widget can render a preview for a file with no open tab). A peek-only
 * model has no tab, no dirty tracking, and a bounded lifetime — editing it via `pushEditOperations`
 * alone would never reach disk and would vanish with the model's TTL. `isPeekPreloadedModel` tells
 * the two apart so this always routes through the file IPC path for a peek-only model instead.
 */
const applyTextEditsToUri = async (
    monaco: Monaco,
    deps: WorkspaceEditApplierDeps,
    uri: string,
    edits: readonly TextEdit[],
): Promise<WorkspaceEditApplyResult> => {
    if (edits.length === 0) return APPLIED

    const path = uriToPath(monaco, uri)
    const model = monaco.editor.getModel(monaco.Uri.parse(uri))
    const isOpenModel = model !== null && !isPeekPreloadedModel(path)

    try {
        if (isOpenModel) {
            model.pushEditOperations(
                [],
                edits.map((edit) => ({ range: lspRangeToMonaco(edit.range), text: edit.newText })),
                () => null,
            )
            /**
             * A model with no editor currently attached is a background tab (never disposed by
             * `model-registry.ts`, so it keeps being "open" long after its `EditorPane` unmounts).
             * Its own `onDidChangeModelContent` → dirty-tracking chain only runs while an editor is
             * attached, so this edit would otherwise go unnoticed until something else overwrites
             * the model with stale disk content on tab activation. Marking it here is how
             * `editor-pane.tsx`'s activation-sync effect finds out not to do that.
             */
            const isAttachedToAnEditor = monaco.editor.getEditors().some((editor) => editor.getModel() === model)
            if (!isAttachedToAnEditor) markModelDirtyExternally(path)
            return APPLIED
        }

        const file = await deps.openFile(path)
        if (file.tier === 'refused') return { applied: false, failureReason: `cannot edit refused file (too large or binary): ${path}` }

        const nextContent = applyTextEditsToContent(file.content, edits)
        await deps.saveFile({ path, content: nextContent })
        /**
         * Keeps a still-live peek-preview model (one the user may currently be looking at, or that
         * a later tab-open within its TTL will adopt via `model-registry.ts`) in sync with what was
         * just written to disk — otherwise it would keep showing/handing off pre-edit content.
         */
        if (model && model.getValue() !== nextContent) model.setValue(nextContent)
        return APPLIED
    } catch (error) {
        return failure(error)
    }
}

const applyCreateFile = async (monaco: Monaco, deps: WorkspaceEditApplierDeps, operation: CreateFile): Promise<WorkspaceEditApplyResult> => {
    const path = uriToPath(monaco, operation.uri)
    try {
        if (operation.options?.overwrite) {
            await deps.saveFile({ path, content: '' })
            return APPLIED
        }
        try {
            await deps.createEntry({ path, isDir: false })
            return APPLIED
        } catch (error) {
            if (operation.options?.ignoreIfExists && isAlreadyExistsError(error)) return APPLIED
            throw error
        }
    } catch (error) {
        return failure(error)
    }
}

const applyRenameFile = async (monaco: Monaco, deps: WorkspaceEditApplierDeps, operation: RenameFile): Promise<WorkspaceEditApplyResult> => {
    const from = uriToPath(monaco, operation.oldUri)
    const to = uriToPath(monaco, operation.newUri)
    try {
        if (!operation.options?.overwrite && (await pathExists(deps, to))) {
            if (operation.options?.ignoreIfExists) return APPLIED
            return { applied: false, failureReason: `already exists: ${to}` }
        }
        await deps.renameEntry({ from, to })
        return APPLIED
    } catch (error) {
        return failure(error)
    }
}

const applyDeleteFile = async (monaco: Monaco, deps: WorkspaceEditApplierDeps, operation: DeleteFile): Promise<WorkspaceEditApplyResult> => {
    const path = uriToPath(monaco, operation.uri)
    try {
        if (operation.options?.ignoreIfNotExists && !(await pathExists(deps, path))) return APPLIED
        await deps.deleteEntry(path)
        return APPLIED
    } catch (error) {
        return failure(error)
    }
}

const isTextDocumentEdit = (operation: DocumentChangeOperation): operation is TextDocumentEdit => !('kind' in operation)

const targetUrisOf = (operation: DocumentChangeOperation): string[] => {
    if (isTextDocumentEdit(operation)) return [operation.textDocument.uri]
    if (operation.kind === 'rename') return [operation.oldUri, operation.newUri]
    return [operation.uri]
}

const applyDocumentChangeOperation = async (
    monaco: Monaco,
    deps: WorkspaceEditApplierDeps,
    operation: DocumentChangeOperation,
    options: WorkspaceEditApplyOptions,
): Promise<WorkspaceEditApplyResult> => {
    const rootFailure = assertPathsWithinRoot(monaco, options.allowedRoot, targetUrisOf(operation))
    if (rootFailure) return rootFailure

    if (isTextDocumentEdit(operation)) {
        const staleFailure = assertVersionCurrent(operation.textDocument, options.getDocumentVersion)
        if (staleFailure) return staleFailure
        return applyTextEditsToUri(monaco, deps, operation.textDocument.uri, operation.edits)
    }
    if (operation.kind === 'create') return applyCreateFile(monaco, deps, operation)
    if (operation.kind === 'rename') return applyRenameFile(monaco, deps, operation)
    return applyDeleteFile(monaco, deps, operation)
}

/**
 * Applies an LSP `WorkspaceEdit` end to end: `documentChanges` (when present, per spec precedence
 * over `changes`) or `changes`, in array order — resource operations (create/rename/delete) must
 * run in the order the server declared them (e.g. create-then-edit). An already-open monaco model
 * is edited via `pushEditOperations` (participates in undo, dirty tracking, LSP `didChange` sync);
 * anything else is read/patched/written through the existing file IPC, sidestepping monaco's
 * `StandaloneBulkEditService` (which throws for models it hasn't loaded and never touches disk).
 * Stops at the first failed operation and reports it — this module intentionally does not attempt
 * rollback of operations already applied.
 */
export const applyWorkspaceEdit = async (
    monaco: Monaco,
    edit: WorkspaceEdit,
    deps: WorkspaceEditApplierDeps = defaultDeps,
    options: WorkspaceEditApplyOptions = {},
): Promise<WorkspaceEditApplyResult> => {
    const operations = edit.documentChanges

    if (operations) {
        for (const operation of operations) {
            const result = await applyDocumentChangeOperation(monaco, deps, operation, options)
            if (!result.applied) return result
        }
        return APPLIED
    }

    for (const [uri, edits] of Object.entries(edit.changes ?? {})) {
        const rootFailure = assertPathsWithinRoot(monaco, options.allowedRoot, [uri])
        if (rootFailure) return rootFailure
        const result = await applyTextEditsToUri(monaco, deps, uri, edits)
        if (!result.applied) return result
    }
    return APPLIED
}
