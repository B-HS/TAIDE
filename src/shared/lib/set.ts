export const toggleInSet = <T>(set: Set<T>, value: T) => {
    const next = new Set(set)
    if (next.has(value)) {
        next.delete(value)
    } else {
        next.add(value)
    }
    return next
}
