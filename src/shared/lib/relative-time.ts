const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60
const HOURS_PER_DAY = 24

export type RelativeTimeToken = { key: string; params: { n: number } | undefined }

export const relativeTimeToken = (timeUnix: number): RelativeTimeToken => {
    const diffSeconds = Math.max(0, Math.floor(Date.now() / 1000) - timeUnix)
    const diffMinutes = Math.floor(diffSeconds / SECONDS_PER_MINUTE)
    const diffHours = Math.floor(diffMinutes / MINUTES_PER_HOUR)
    const diffDays = Math.floor(diffHours / HOURS_PER_DAY)
    if (diffDays > 0) return { key: 'git.timeDaysAgo', params: { n: diffDays } }
    if (diffHours > 0) return { key: 'git.timeHoursAgo', params: { n: diffHours } }
    if (diffMinutes > 0) return { key: 'git.timeMinutesAgo', params: { n: diffMinutes } }
    return { key: 'git.timeJustNow', params: undefined }
}
