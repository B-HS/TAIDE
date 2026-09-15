import { i18next } from '@shared/i18n/i18n'

const MS_PER_SECOND = 1_000
const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60

/**
 * How long something ran, as the one short phrase a notification banner has room for — "45s",
 * "3m 12s", "1h 2m".
 *
 * Two units at most, and never the smallest one below the largest: once a run is measured in hours
 * the seconds are noise, and the value exists to answer "was I away five minutes or an hour", not
 * to be a stopwatch readout. Sub-second runs round down to "0s" rather than growing a millisecond
 * unit — nothing reaches here that was not already over a ten-second threshold
 * (`shared/constants/notification.ts`), so that case only arises in tests.
 *
 * Reads `i18next` directly rather than taking a `t` for the same reason `ipc-error-message.ts`
 * does: every call site is a hook body or an event handler building a string for the OS, not a
 * React render that would need to re-run on a language change.
 */
export const formatDurationShort = (durationMs: number) => {
    const totalSeconds = Math.max(0, Math.floor(durationMs / MS_PER_SECOND))
    const totalMinutes = Math.floor(totalSeconds / SECONDS_PER_MINUTE)
    const hours = Math.floor(totalMinutes / MINUTES_PER_HOUR)
    const minutes = totalMinutes % MINUTES_PER_HOUR
    const seconds = totalSeconds % SECONDS_PER_MINUTE

    if (hours > 0) return i18next.t('common.durationHoursMinutes', { hours, minutes })
    if (totalMinutes > 0) return i18next.t('common.durationMinutesSeconds', { minutes, seconds })
    return i18next.t('common.durationSeconds', { seconds })
}
