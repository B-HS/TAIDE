export type EntryNameErrorKey = 'explorer.entryNameReserved' | 'explorer.entryNameInvalidChar' | 'explorer.entryNameDuplicate'

const PATH_SEPARATOR = '/'
const RESERVED_SEGMENT_NAMES = new Set(['.', '..'])
const INVALID_SEGMENT_CHAR_PATTERN = /[\\:*?"<>|]/
const TRAILING_DOT_PATTERN = /\.$/

/**
 * Resolves the directory a nested entry name (`sub/dir/a.ts`) is actually created in,
 * so duplicate checks compare against the real siblings instead of the base directory's.
 */
export const resolveEntryParentDir = (parentDir: string, name: string) => {
    const segments = name
        .trim()
        .split(PATH_SEPARATOR)
        .filter((segment) => segment.length > 0)
    if (segments.length <= 1) return parentDir

    const base = parentDir.endsWith(PATH_SEPARATOR) ? parentDir.slice(0, -1) : parentDir
    return [base, ...segments.slice(0, -1)].join(PATH_SEPARATOR)
}

export const validateEntryName = (name: string, siblingNames: string[]): EntryNameErrorKey | null => {
    const trimmed = name.trim()
    if (!trimmed) return null

    const segments = trimmed.split(PATH_SEPARATOR).filter((segment) => segment.length > 0)
    const lastSegment = segments.at(-1)
    if (!lastSegment) return 'explorer.entryNameInvalidChar'

    if (RESERVED_SEGMENT_NAMES.has(lastSegment)) return 'explorer.entryNameReserved'
    if (INVALID_SEGMENT_CHAR_PATTERN.test(lastSegment) || TRAILING_DOT_PATTERN.test(lastSegment)) return 'explorer.entryNameInvalidChar'
    if (siblingNames.includes(lastSegment)) return 'explorer.entryNameDuplicate'
    return null
}
