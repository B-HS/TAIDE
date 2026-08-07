const splitBaseAndExtension = (name: string) => {
    const dotIndex = name.lastIndexOf('.')
    if (dotIndex <= 0) return { base: name, extension: '' }
    return { base: name.slice(0, dotIndex), extension: name.slice(dotIndex) }
}

export const buildUniqueEntryName = (desiredName: string, existingNames: string[], conflictSuffix: string) => {
    const existing = new Set(existingNames)
    if (!existing.has(desiredName)) return desiredName

    const { base, extension } = splitBaseAndExtension(desiredName)
    let attempt = 1
    let candidate = `${base} ${conflictSuffix}${extension}`
    while (existing.has(candidate)) {
        attempt += 1
        candidate = `${base} ${conflictSuffix} ${attempt}${extension}`
    }
    return candidate
}
