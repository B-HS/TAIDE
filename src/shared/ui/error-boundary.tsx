import { Component, type ErrorInfo, type PropsWithChildren } from 'react'
import { withTranslation, type WithTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'

type ErrorBoundaryOwnProps = PropsWithChildren<{
    labelKey: string
}>

type ErrorBoundaryProps = ErrorBoundaryOwnProps & WithTranslation

type ErrorBoundaryState = {
    error: Error | null
}

const INITIAL_ERROR_BOUNDARY_STATE: ErrorBoundaryState = { error: null }

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
 * re-renders if the active locale changes while it happens to be showing.
 *
 * Retry resets `state.error` to `null`, re-rendering `children`. This is a genuine fresh mount, not
 * a stale reuse of the crashed instance: React already fully unmounted the thrown subtree's fiber
 * the moment it caught the error (that is how error boundaries work), so there is nothing left to
 * "resume" — the next render creates brand-new fibers for `children` from scratch.
 */
/**
 * Exported alongside {@link ErrorBoundary} (the `withTranslation()`-wrapped component callers use)
 * only so `error-boundary.test.ts` can exercise `getDerivedStateFromError`/`render`/`handleRetry`
 * directly — the HOC wrapper calls hooks internally and so cannot be invoked outside of an actual
 * React render, and this project has no DOM render harness (RTL/jsdom) to drive one. See that
 * test file's top-of-file comment for the resulting scope and limits.
 */
export class ErrorBoundaryBase extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
    state = INITIAL_ERROR_BOUNDARY_STATE

    static getDerivedStateFromError(error: Error) {
        return { error }
    }

    componentDidCatch(error: Error, errorInfo: ErrorInfo) {
        console.error(error, errorInfo)
    }

    handleRetry = () => this.setState(INITIAL_ERROR_BOUNDARY_STATE)

    render() {
        const { error } = this.state
        if (!error) return this.props.children

        const { t, labelKey } = this.props
        return (
            <div
                role='alert'
                className='bg-app-background text-app-foreground flex h-full w-full flex-col items-center justify-center gap-2 p-4 text-center text-sm'>
                <span>{t(labelKey)}</span>
                <span className='text-app-sidebar-icon-default text-xs'>{t('errorBoundary.description')}</span>
                {import.meta.env.DEV && error.stack && (
                    <pre className='text-status-error max-h-32 max-w-full overflow-auto text-left text-[10px]'>{error.stack}</pre>
                )}
                <Button type='button' variant='outline' size='xs' onClick={this.handleRetry}>
                    {t('common.retry')}
                </Button>
            </div>
        )
    }
}

export const ErrorBoundary = withTranslation()(ErrorBoundaryBase)
