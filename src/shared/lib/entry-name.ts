export type EntryNameErrorKey = 'explorer.entryNameReserved' | 'explorer.entryNameInvalidChar' | 'explorer.entryNameDuplicate'

const RESERVED_SEGMENT_NAMES = new Set(['.', '..'])
const INVALID_SEGMENT_CHAR_PATTERN = /[\\:*?"<>|]/
const TRAILING_DOT_PATTERN = /\.$/

export const validateEntryName = (name: string, siblingNames: string[]): EntryNameErrorKey | null => {
    const trimmed = name.trim()
    if (!trimmed) return null

    const segments = trimmed.split('/').filter((segment) => segment.length > 0)
    const lastSegment = segments.at(-1)
    if (!lastSegment) return 'explorer.entryNameInvalidChar'

    if (RESERVED_SEGMENT_NAMES.has(lastSegment)) return 'explorer.entryNameReserved'
    if (INVALID_SEGMENT_CHAR_PATTERN.test(lastSegment) || TRAILING_DOT_PATTERN.test(lastSegment)) return 'explorer.entryNameInvalidChar'
    if (siblingNames.includes(lastSegment)) return 'explorer.entryNameDuplicate'
    return null
}
