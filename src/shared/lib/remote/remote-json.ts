export const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null

export const parseJson = (text: string): unknown => {
    try {
        return JSON.parse(text)
    } catch {
        return null
    }
}

export const numberOf = (value: unknown) => (typeof value === 'number' ? value : 0)

export const stringOf = (value: unknown) => (typeof value === 'string' ? value : '')
