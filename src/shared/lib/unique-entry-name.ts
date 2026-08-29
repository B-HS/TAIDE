export type UniqueEntryNameKind = 'file' | 'directory'

/**
 * Directories have no extension: a folder named `v1.2` must stay `v1.2 copy`, never `v1 copy.2`.
 * Only a file name's trailing `.ext` is preserved around the conflict suffix.
 */
const splitBaseAndExtension = (name: string, kind: UniqueEntryNameKind) => {
    const dotIndex = name.lastIndexOf('.')
    if (kind === 'directory' || dotIndex <= 0) return { base: name, extension: '' }
    return { base: name.slice(0, dotIndex), extension: name.slice(dotIndex) }
}

export const buildUniqueEntryName = (desiredName: string, existingNames: string[], conflictSuffix: string, kind: UniqueEntryNameKind = 'file') => {
    const existing = new Set(existingNames)
    if (!existing.has(desiredName)) return desiredName

    const { base, extension } = splitBaseAndExtension(desiredName, kind)
    let attempt = 1
    let candidate = `${base} ${conflictSuffix}${extension}`
    while (existing.has(candidate)) {
        attempt += 1
        candidate = `${base} ${conflictSuffix} ${attempt}${extension}`
    }
    return candidate
}
