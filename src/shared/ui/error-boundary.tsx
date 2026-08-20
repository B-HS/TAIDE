import { Component, createRef, type ErrorInfo, type PropsWithChildren } from 'react'
import { withTranslation, type WithTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'

type ErrorBoundaryOwnProps = PropsWithChildren<{
    labelKey: string
    labelFallback?: string
    fallbackSizeClassName?: string
    onCaught?: (error: unknown, errorInfo: ErrorInfo) => void
}>

type ErrorBoundaryProps = ErrorBoundaryOwnProps & WithTranslation

type ErrorBoundaryState = {
    hasError: boolean
    error: unknown
}

const INITIAL_ERROR_BOUNDARY_STATE: ErrorBoundaryState = { hasError: false, error: null }
const DEFAULT_FALLBACK_SIZE_CLASS_NAME = 'h-full w-full'

/**
 * Exported alongside {@link ErrorBoundary} (the `withTranslation()`-wrapped component callers use)
 * only so `error-boundary.test.tsx` can exercise `getDerivedStateFromError`/`render`/`handleRetry`/
 * `componentDidCatch` directly — the HOC wrapper calls hooks internally and so cannot be invoked
 * outside of an actual React render, and this project has no DOM render harness (RTL/jsdom) to
 * drive one. This is a deliberate, narrow exception to the one-file-one-component rule (fsd.md §3):
 * the raw class has exactly one consumer (that test file), never imported by application code. See
 * that test file's top-of-file comment for the resulting scope and limits.
 */
export class ErrorBoundaryBase extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
    state = INITIAL_ERROR_BOUNDARY_STATE

    fallbackRef = createRef<HTMLDivElement>()

    static getDerivedStateFromError(error: unknown) {
        return { hasError: true, error }
    }

    componentDidCatch(error: unknown, errorInfo: ErrorInfo) {
        console.error(error, errorInfo)
        this.props.onCaught?.(error, errorInfo)
    }

    componentDidMount() {
        if (this.state.hasError) this.fallbackRef.current?.focus()
    }

    componentDidUpdate(_prevProps: ErrorBoundaryProps, prevState: ErrorBoundaryState) {
        if (prevState.hasError || !this.state.hasError) return
        this.fallbackRef.current?.focus()
    }

    handleRetry = () => this.setState(INITIAL_ERROR_BOUNDARY_STATE)

    render() {
        const { hasError, error } = this.state
        if (!hasError) return this.props.children

        const { t, labelKey, labelFallback, fallbackSizeClassName = DEFAULT_FALLBACK_SIZE_CLASS_NAME } = this.props
        return (
            <div
                ref={this.fallbackRef}
                role='alert'
                tabIndex={-1}
                className={cn(
                    'bg-app-background text-app-foreground flex flex-col items-center justify-center gap-2 p-4 text-center text-sm',
                    fallbackSizeClassName,
                )}>
                <span>{t(labelKey, { defaultValue: labelFallback })}</span>
                <span className='text-app-sidebar-icon-default text-xs'>
                    {t('errorBoundary.description', { defaultValue: 'Something went wrong in this area.' })}
                </span>
                {import.meta.env.DEV && error instanceof Error && error.stack && (
                    <pre className='text-status-error max-h-32 max-w-full overflow-auto text-left text-[10px]'>{error.stack}</pre>
                )}
                <Button type='button' variant='outline' size='xs' onClick={this.handleRetry}>
                    {t('common.retry', { defaultValue: 'Retry' })}
                </Button>
            </div>
        )
    }
}

/**
 * Region-scoped crash containment (crash-class-seal-contract.md §1-2): catches a render or effect
 * error thrown anywhere inside `children` and replaces just that subtree with a labeled fallback
 * and a retry button, instead of letting the error propagate to the React root and unmount the
 * whole app (React 19's default behavior with no boundary in the way).
 *
 * `labelKey` is a locale-catalog key, not pre-resolved text — the caller names which region this
 * boundary wraps (e.g. `errorBoundary.editorArea`, or an existing key like `explorer.title`) and
 * this component resolves it itself, since a class component cannot call `useTranslation()`
 * directly. Wrapped with `withTranslation()` (not a plain `i18next.t()` call) so the fallback also
 * re-renders if the active locale changes while it happens to be showing. `labelFallback` (and the
 * two other fixed strings' own inline defaults) covers the locale-catalog-not-yet-loaded window —
 * see `onCaught` below — so the fallback never shows a raw `errorBoundary.*` key.
 *
 * `fallbackSizeClassName` lets a caller override the default `h-full w-full` sizing when the
 * boundary replaces a region with its own fixed cross-axis size in a flex layout (e.g. a `w-14
 * shrink-0` rail or a `h-6 shrink-0` bar) — without it, the fallback's implicit 100% flex-basis
 * collapses the sibling that shares that flex container down to 0px (crash-class-seal-contract.md
 * §4). Regions that already expect a 100%-of-slot child (the root boundary, and the Panel-hosted
 * explorer/editor boundaries) leave it unset.
 *
 * `onCaught` is an escape hatch for the one boundary that needs it: the root boundary in
 * `main.tsx` wraps `<App/>` above `ThemeProvider`/`LocaleProvider`, so a crash before either has
 * settled leaves `document.documentElement`'s `data-theme-ready`/`data-locale-ready` gate
 * (global.css) permanently unset and, for an auxiliary window, its OS window never shown — the
 * exact "blank window" symptom this component exists to prevent (crash-class-seal-contract.md §4).
 * `main.tsx` passes a callback that sets both dataset flags and calls `getCurrentWindow().show()`;
 * this component stays free of any Tauri import so `shared/ui` doesn't reach sideways into a
 * platform API. Every other boundary (`app-shell.tsx`, `auxiliary-window-shell.tsx`) mounts only
 * as a descendant of an already-settled `ThemeProvider`/`LocaleProvider`, so passing `onCaught`
 * there would just re-run an already-harmless no-op — they leave it unset.
 *
 * Retry resets `state.hasError`/`state.error`, re-rendering `children`. This is a genuine fresh
 * mount, not a stale reuse of the crashed instance: React already fully unmounted the thrown
 * subtree's fiber the moment it caught the error (that is how error boundaries work), so there is
 * nothing left to "resume" — the next render creates brand-new fibers for `children` from scratch.
 * On the transition into the fallback (or on mount, for a crash during this boundary's own first
 * render), focus moves to the fallback container so keyboard/screen-reader users aren't left
 * wondering where a focused subtree (e.g. a focused monaco editor) went.
 */
export const ErrorBoundary = withTranslation()(ErrorBoundaryBase)
