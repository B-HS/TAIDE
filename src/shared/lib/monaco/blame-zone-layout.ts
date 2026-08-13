const BLAME_FONT_SIZE_RATIO = 0.9
const BLAME_MIN_LINE_HEIGHT_FACTOR = 1.3
const BLAME_ZONE_MIN_AFTER_LINE_NUMBER = 0

export const computeBlameZoneFontSize = (editorFontSize: number) => (editorFontSize * BLAME_FONT_SIZE_RATIO) | 0

export const computeBlameZoneHeightPx = (editorFontSize: number, editorLineHeight: number) => {
    const lineHeightFactor = Math.max(BLAME_MIN_LINE_HEIGHT_FACTOR, editorLineHeight / editorFontSize)
    return (computeBlameZoneFontSize(editorFontSize) * lineHeightFactor) | 0
}

export const computeBlameZoneAfterLineNumber = (line: number) => Math.max(BLAME_ZONE_MIN_AFTER_LINE_NUMBER, line - 1)
