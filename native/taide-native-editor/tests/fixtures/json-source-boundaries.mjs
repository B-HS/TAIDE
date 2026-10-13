export const writeBoundedScanner = async (sourceUrl, destinationUrl) => {
    const wrapper = `import { createScanner as original } from ${JSON.stringify(sourceUrl.href)}
export const createScanner = (text, ...args) => {
    const scanner = original(text, ...args)
    const start = () => Math.min(text.length, scanner.getTokenOffset())
    const end = () => {
        const offset = scanner.getTokenOffset() + scanner.getTokenLength()
        if (offset > text.length && (offset !== text.length + 1 || ![13, 17].includes(scanner.getToken()))) throw new Error('Unexpected JSON source scanner overflow')
        return Math.min(text.length, offset)
    }
    return { ...scanner, getTokenOffset: start, getTokenLength: () => end() - start() }
}
`
    await Bun.write(destinationUrl, wrapper)
}
