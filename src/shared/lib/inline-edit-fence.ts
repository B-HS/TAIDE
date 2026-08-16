const CODE_FENCE_MARKER = '```'

/**
 * Strips a markdown code fence from an AI response leniently — the response doesn't have to be
 * *only* a fence block (models routinely ignore a "no explanations" instruction and add a leading
 * "Here's the code:" or a trailing "I renamed x to y." line). Leading prose before the opening
 * fence and trailing prose after the closing fence are both discarded; only the text between the
 * first opening fence's own line (skipping an optional language tag) and the last fence marker in
 * the response is kept. Fences nested inside that span (a fenced example inside a fenced response)
 * are left untouched, since only the first/last markers are treated as the outer pair. A single-line
 * response with both markers on the same line (no language tag possible) is also handled — the
 * content is everything between the two markers. Falls back to the trimmed input as-is when there's
 * no fence, or the fence is unterminated (only one ``` marker in the whole response).
 */
export const stripCodeFence = (text: string) => {
    const trimmed = text.trim()
    const openStart = trimmed.indexOf(CODE_FENCE_MARKER)
    if (openStart === -1) return trimmed

    const closeStart = trimmed.lastIndexOf(CODE_FENCE_MARKER)
    if (closeStart <= openStart) return trimmed

    const openLineEnd = trimmed.indexOf('\n', openStart)
    const contentStart = openLineEnd !== -1 && openLineEnd < closeStart ? openLineEnd + 1 : openStart + CODE_FENCE_MARKER.length

    return trimmed.slice(contentStart, closeStart).replace(/\r\n/g, '\n').replace(/\n$/, '')
}
