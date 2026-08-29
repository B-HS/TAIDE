import type { ReplaceSkippedFile, ReplaceSkipReason, SearchReplaceResult } from '@shared/api/bindings'

/**
 * How many skipped files are named in the toast before it falls back to "and N more". The backend
 * already caps the list it sends at `REPLACE_SKIP_REPORT_LIMIT` (50) while `skippedCount` stays the
 * true total, so both cut-offs have to be accounted for when phrasing the remainder.
 */
export const REPLACE_SKIP_LIST_LIMIT = 5

export const REPLACE_SKIP_REASON_MESSAGE_KEY: Record<ReplaceSkipReason, string> = {
    tooLarge: 'search.replaceSkipReason.tooLarge',
    binary: 'search.replaceSkipReason.binary',
    notUtf8: 'search.replaceSkipReason.notUtf8',
    unreadable: 'search.replaceSkipReason.unreadable',
    writeFailed: 'search.replaceSkipReason.writeFailed',
}

/**
 * Turns a replace result's skip fields into the shape a toast renders, or `null` when nothing was
 * skipped. Before the backend reported skips, a replace whose every target was refused (oversized,
 * binary, non-UTF-8, unwritable) announced the exact same "0 files changed" as a query that simply
 * had no matches — audit §4-B C10.
 */
export const buildReplaceSkipReport = (
    result: Pick<SearchReplaceResult, 'skipped' | 'skippedCount'>,
    describe: (file: ReplaceSkippedFile) => string,
) => {
    if (result.skippedCount <= 0) return null

    const listed = result.skipped.slice(0, REPLACE_SKIP_LIST_LIMIT)

    return { total: result.skippedCount, lines: listed.map(describe), remaining: result.skippedCount - listed.length }
}
