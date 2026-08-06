import type { BlameLine } from '@shared/api/bindings'

const BLAME_MESSAGE_MAX_LENGTH = 50
const SECONDS_PER_MINUTE = 60
const SECONDS_PER_HOUR = 60 * 60
const SECONDS_PER_DAY = 60 * 60 * 24
const SECONDS_PER_MONTH = 60 * 60 * 24 * 30
const SECONDS_PER_YEAR = 60 * 60 * 24 * 365

const pluralize = (count: number, unit: string) => `${count} ${unit}${count === 1 ? '' : 's'} ago`

export const formatRelativeTime = (timeUnixSeconds: number | null, nowMs: number) => {
    if (timeUnixSeconds === null) return 'unknown time'

    const diffSeconds = Math.max(0, Math.floor(nowMs / 1000 - timeUnixSeconds))
    if (diffSeconds < SECONDS_PER_MINUTE) return 'just now'
    if (diffSeconds < SECONDS_PER_HOUR) return pluralize(Math.floor(diffSeconds / SECONDS_PER_MINUTE), 'minute')
    if (diffSeconds < SECONDS_PER_DAY) return pluralize(Math.floor(diffSeconds / SECONDS_PER_HOUR), 'hour')
    if (diffSeconds < SECONDS_PER_MONTH) return pluralize(Math.floor(diffSeconds / SECONDS_PER_DAY), 'day')
    if (diffSeconds < SECONDS_PER_YEAR) return pluralize(Math.floor(diffSeconds / SECONDS_PER_MONTH), 'month')
    return pluralize(Math.floor(diffSeconds / SECONDS_PER_YEAR), 'year')
}

export const truncateBlameMessage = (message: string) => {
    const singleLine = message.replace(/\s+/g, ' ').trim()
    if (singleLine.length <= BLAME_MESSAGE_MAX_LENGTH) return singleLine
    return `${singleLine.slice(0, BLAME_MESSAGE_MAX_LENGTH).trimEnd()}...`
}

export const formatBlameLine = (blame: BlameLine, nowMs: number, currentUser?: string | null) => {
    if (blame.isUncommitted) return 'You, now - Uncommitted changes'

    const author = currentUser && blame.author === currentUser ? 'You' : blame.author
    const agoText = formatRelativeTime(blame.timeUnix, nowMs)
    const messageText = truncateBlameMessage(blame.summary)
    return `${author}, ${agoText} • ${messageText}`
}
