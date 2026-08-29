import type { FileTreeNodeKind } from '@features/explorer/file-tree-row'
import { isIpcErrorKey } from '@shared/lib/ipc-error-message'
import { fileNameOf } from '@shared/lib/relative-path'
import { buildUniqueEntryName } from '@shared/lib/unique-entry-name'
import { parentDirOf } from '@widgets/explorer/explorer-path'

export type ExplorerClipboardEntry = { mode: 'cut' | 'copy'; path: string; kind: FileTreeNodeKind }

/** Backend guard `file_rename`/`file_copy` raise when the destination already holds an entry (d-50 S2). */
export const DESTINATION_EXISTS_ERROR_KEY = 'error.file.destinationExists'

/**
 * How many destination names one paste may try before giving up and surfacing the backend error.
 * Each retry is one full IPC round trip, and every attempt past the first means the visible-rows
 * sibling scan missed that many hidden entries — a bounded loop keeps a pathological directory
 * (thousands of `name copy N` siblings under a collapsed folder) from spinning forever.
 */
export const PASTE_DESTINATION_ATTEMPT_LIMIT = 8

/**
 * Cutting an entry and pasting it back into the folder it already lives in is a no-op, not a
 * duplication — without this the unique-name pass would rename the entry to "name copy" in place,
 * because the entry itself is one of the sibling names the candidate is checked against.
 */
export const isSamePlaceCutPaste = (clipboard: ExplorerClipboardEntry, targetDir: string) =>
    clipboard.mode === 'cut' && parentDirOf(clipboard.path) === targetDir

type PasteWithUniqueEntryNameInput = {
    clipboard: ExplorerClipboardEntry
    siblingNames: string[]
    conflictSuffix: string
    run: (destinationName: string) => Promise<void>
    attemptLimit?: number
}

/**
 * Runs `run` against the first destination name that the backend accepts. `siblingNames` can only
 * carry the *visible* tree rows (the explorer tree is lazy — a collapsed folder's children were
 * never fetched), so the locally-computed unique name can still collide on disk; the backend then
 * answers with {@link DESTINATION_EXISTS_ERROR_KEY} instead of overwriting, and each rejected name
 * is folded back into the taken set for the next candidate. Any other failure propagates
 * unchanged — only a destination collision is retryable.
 */
export const pasteWithUniqueEntryName = async ({
    clipboard,
    siblingNames,
    conflictSuffix,
    run,
    attemptLimit = PASTE_DESTINATION_ATTEMPT_LIMIT,
}: PasteWithUniqueEntryNameInput) => {
    const entryName = fileNameOf(clipboard.path)
    const takenNames = [...siblingNames]
    let lastError: unknown = null

    for (let attempt = 0; attempt < attemptLimit; attempt += 1) {
        const destinationName = buildUniqueEntryName(entryName, takenNames, conflictSuffix, clipboard.kind)
        try {
            await run(destinationName)
            return destinationName
        } catch (error) {
            if (!isIpcErrorKey(error, DESTINATION_EXISTS_ERROR_KEY)) throw error
            lastError = error
            takenNames.push(destinationName)
        }
    }

    throw lastError
}
