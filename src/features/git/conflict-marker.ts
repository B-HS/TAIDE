export type ConflictRegion = {
    startLine: number
    baseLine: number | null
    separatorLine: number
    endLine: number
    oursLabel: string
    theirsLabel: string
}

const CONFLICT_START_MARKER = '<<<<<<<'
const CONFLICT_BASE_MARKER = '|||||||'
const CONFLICT_SEPARATOR_MARKER = '======='
const CONFLICT_END_MARKER = '>>>>>>>'

const stripTrailingCarriageReturn = (line: string) => (line.endsWith('\r') ? line.slice(0, -1) : line)

const markerLabel = (line: string, marker: string) => line.slice(marker.length).trim()

type ParseState =
    | { kind: 'seekStart' }
    | { kind: 'seekSeparator'; startLine: number; baseLine: number | null; oursLabel: string }
    | { kind: 'seekEnd'; startLine: number; baseLine: number | null; separatorLine: number; oursLabel: string }

/**
 * Parses git conflict markers (`<<<<<<<` / `|||||||` / `=======` / `>>>>>>>`) out of file
 * content into per-region line boundaries. An unterminated or malformed region (a start marker
 * with no matching separator/end before EOF or before the next start marker) is silently
 * dropped rather than throwing — the caller only ever sees fully-formed regions.
 */
export const parseConflictMarkers = (content: string): ConflictRegion[] => {
    const regions: ConflictRegion[] = []
    let state: ParseState = { kind: 'seekStart' }

    content.split('\n').forEach((rawLine, index) => {
        const line = stripTrailingCarriageReturn(rawLine)
        const lineNumber = index + 1

        if (line.startsWith(CONFLICT_START_MARKER)) {
            state = { kind: 'seekSeparator', startLine: lineNumber, baseLine: null, oursLabel: markerLabel(line, CONFLICT_START_MARKER) }
            return
        }

        if (state.kind === 'seekSeparator' && line.startsWith(CONFLICT_BASE_MARKER)) {
            state = { ...state, baseLine: lineNumber }
            return
        }

        if (state.kind === 'seekSeparator' && line === CONFLICT_SEPARATOR_MARKER) {
            state = { kind: 'seekEnd', startLine: state.startLine, baseLine: state.baseLine, separatorLine: lineNumber, oursLabel: state.oursLabel }
            return
        }

        if (state.kind === 'seekEnd' && line.startsWith(CONFLICT_END_MARKER)) {
            regions.push({
                startLine: state.startLine,
                baseLine: state.baseLine,
                separatorLine: state.separatorLine,
                endLine: lineNumber,
                oursLabel: state.oursLabel,
                theirsLabel: markerLabel(line, CONFLICT_END_MARKER),
            })
            state = { kind: 'seekStart' }
        }
    })

    return regions
}

const sliceLinesInclusive = (lines: string[], fromLineInclusive: number, toLineInclusive: number) =>
    fromLineInclusive > toLineInclusive ? [] : lines.slice(fromLineInclusive - 1, toLineInclusive)

const oursLines = (lines: string[], region: ConflictRegion) =>
    sliceLinesInclusive(lines, region.startLine + 1, (region.baseLine ?? region.separatorLine) - 1)

const theirsLines = (lines: string[], region: ConflictRegion) => sliceLinesInclusive(lines, region.separatorLine + 1, region.endLine - 1)

const replaceRegionLines = (lines: string[], region: ConflictRegion, replacement: string[]) => [
    ...lines.slice(0, region.startLine - 1),
    ...replacement,
    ...lines.slice(region.endLine),
]

/** Replaces `region` (markers included) with only the "ours"/current side's content. */
export const acceptCurrentChange = (content: string, region: ConflictRegion) => {
    const lines = content.split('\n')
    return replaceRegionLines(lines, region, oursLines(lines, region)).join('\n')
}

/** Replaces `region` (markers included) with only the "theirs"/incoming side's content. */
export const acceptIncomingChange = (content: string, region: ConflictRegion) => {
    const lines = content.split('\n')
    return replaceRegionLines(lines, region, theirsLines(lines, region)).join('\n')
}

/** Replaces `region` (markers included) with both sides' content, ours before theirs. */
export const acceptBothChanges = (content: string, region: ConflictRegion) => {
    const lines = content.split('\n')
    return replaceRegionLines(lines, region, [...oursLines(lines, region), ...theirsLines(lines, region)]).join('\n')
}
