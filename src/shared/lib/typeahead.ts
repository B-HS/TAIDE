export type TypeaheadItem = {
    name: string
}

export const findTypeaheadMatchIndex = <T extends TypeaheadItem>(items: T[], buffer: string, fromIndex: number) => {
    if (!buffer || items.length === 0) return -1

    const needle = buffer.toLowerCase()
    for (let offset = 1; offset <= items.length; offset += 1) {
        const index = (fromIndex + offset) % items.length
        if (items[index].name.toLowerCase().startsWith(needle)) return index
    }
    return -1
}
