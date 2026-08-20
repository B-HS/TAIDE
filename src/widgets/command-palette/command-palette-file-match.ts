import { fileNameOf } from '@shared/lib/relative-path'

export type PaletteFileMatchDisplay = {
    fileName: string
    dirPath: string | null
    fileNameIndices: number[]
    dirPathIndices: number[]
}

/**
 * Splits a project-relative file path into a filename (main label) and its parent directory
 * (subtitle), remapping fuzzy-match indices — positions in the full `relativePath` — onto each
 * part so highlighting still lines up after the 2-line split. The path separator itself (if
 * matched) belongs to neither part and is dropped.
 */
export const splitFileMatchForDisplay = (relativePath: string, matchIndices: number[]): PaletteFileMatchDisplay => {
    const separatorIndex = relativePath.lastIndexOf('/')
    const fileName = fileNameOf(relativePath)
    if (separatorIndex === -1) return { fileName, dirPath: null, fileNameIndices: matchIndices, dirPathIndices: [] }

    const dirPathIndices = matchIndices.filter((index) => index < separatorIndex)
    const fileNameIndices = matchIndices.filter((index) => index > separatorIndex).map((index) => index - separatorIndex - 1)

    return { fileName, dirPath: relativePath.slice(0, separatorIndex), fileNameIndices, dirPathIndices }
}
