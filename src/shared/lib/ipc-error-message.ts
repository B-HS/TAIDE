import { i18next } from '@shared/i18n/i18n'
import { IpcError } from '@shared/api/unwrap-result'

export type LocaleKeyError = Error & { localeKey?: string; localeArgs?: Record<string, string> }

/**
 * Structural guard for a boundary-normalized error carrying an optional `localeKey`/`localeArgs`
 * pair — `IpcError` and `RawFileReadError` both shape-match this (both assign the two fields
 * unconditionally in their constructor, even when `undefined`) without either needing to extend
 * the other. Deliberately not `instanceof IpcError`: gating on the class would silently drop any
 * other boundary error that normalizes the same way. Exported so `useIpcErrorMessage`
 * (`@shared/hooks/use-ipc-error-message`) shares the exact same gate.
 */
export const hasLocaleKey = (error: unknown): error is LocaleKeyError => error instanceof Error && 'localeKey' in error

/**
 * Backend error → display text, for call sites outside a component render (toasts, non-reactive
 * catch blocks). A `Localized` backend error resolves through the active locale catalog when its
 * key is registered there (`t(key, args)`); everything else — an error with no `localeKey`, or a
 * value that never reached the IPC boundary at all — falls back to `error.message`/`String(error)`.
 * `i18next.exists` is required here: i18next returns the bare key string for a missing key instead
 * of throwing, so skipping this check would show `error.git.…` literals whenever a user's locale
 * pack lags the catalog (`resolve_pack`'s backfill covers the Rust side; this is the matching
 * frontend guard).
 */
export const describeIpcError = (error: unknown) => {
    if (hasLocaleKey(error) && error.localeKey && i18next.exists(error.localeKey)) return i18next.t(error.localeKey, error.localeArgs ?? {})
    return error instanceof Error ? error.message : String(error)
}

/** Whether `error` is a `Localized` backend error carrying this exact locale key — used to branch on a specific backend condition (e.g. a user-initiated cancellation) instead of showing it as a failure. */
export const isIpcErrorKey = (error: unknown, key: string) => error instanceof IpcError && error.localeKey === key

/**
 * Whether `error` is a backend `NotFound`, regardless of whether it arrived as a bare
 * `AppError::NotFound` or as the `Localized` newtype wrapping one — `normalizeAppError`
 * (`@shared/api/unwrap-result`) flattens the latter's inner `kind` into the same `code`, so both
 * shapes match here. `useOpenFileTab` branches on it to refresh the quick-open file index that
 * handed out a path which no longer exists on disk, instead of only reporting the failure.
 */
export const isNotFoundIpcError = (error: unknown) => error instanceof IpcError && error.code === 'NotFound'
