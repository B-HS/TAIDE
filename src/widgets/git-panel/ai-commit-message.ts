import type { LogEntry } from '@shared/api/bindings'
import { stripCodeFence } from '@shared/lib/inline-edit-fence'

export const RECENT_COMMITS_FOR_AI_CONTEXT_COUNT = 20

const GIT_SHORT_HASH_LENGTH = 7

/**
 * Builds the `{recentCommits}` prompt variable for `ai_commit_message` — the most recent commits
 * (newest first, capped at {@link RECENT_COMMITS_FOR_AI_CONTEXT_COUNT}) as `<shortHash> <summary>`
 * lines, matching how `commit-message-default.json`'s system prompt describes them ("style
 * reference only — never copy their content").
 */
export const buildRecentCommitsSummaryForAi = (log: LogEntry[]) =>
    log
        .slice(0, RECENT_COMMITS_FOR_AI_CONTEXT_COUNT)
        .map((entry) => `${entry.id.slice(0, GIT_SHORT_HASH_LENGTH)} ${entry.summary}`)
        .join('\n')

const SURROUNDING_QUOTE_PAIRS: [string, string][] = [
    ['"', '"'],
    ["'", "'"],
    ['`', '`'],
    ['“', '”'],
    ['‘', '’'],
]

const stripSurroundingQuotes = (text: string) => {
    const pair = SURROUNDING_QUOTE_PAIRS.find(
        ([open, close]) => text.startsWith(open) && text.endsWith(close) && text.length >= open.length + close.length,
    )
    return pair ? text.slice(pair[0].length, text.length - pair[1].length).trim() : text
}

/**
 * Post-processes the raw `ai_commit_message` response text: strips a markdown code fence (some
 * models wrap the message in ``` despite the prompt's "no code fences" instruction — the same
 * lenient `stripCodeFence` Inline Edit uses, so both AI response paths share one fence parser) and
 * a surrounding quote pair, then trims.
 */
export const sanitizeAiCommitMessageResponse = (text: string) => stripSurroundingQuotes(stripCodeFence(text).trim())
