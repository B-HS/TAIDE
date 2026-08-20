import { describe, expect, mock, test } from 'bun:test'
import type { ComponentProps, ErrorInfo, ReactElement } from 'react'
import type { TFunction } from 'i18next'
import { ErrorBoundaryBase } from '@shared/ui/error-boundary'

/**
 * This project has no DOM render harness (no `@testing-library/react`, no jsdom/happy-dom — the
 * same constraint `code-editor-visibility.test.ts` documents for monaco-backed components), so
 * `<ErrorBoundary>` itself — the `withTranslation()`-wrapped component callers actually use —
 * cannot be mounted here: the HOC calls `useContext` internally, which throws outside of an active
 * React render. These tests instead exercise `ErrorBoundaryBase` (the underlying class,
 * unexported by default from a caller's perspective but exported alongside `ErrorBoundary`
 * specifically for this file) directly:
 *
 * - `getDerivedStateFromError` is a `static` method — callable with no instance at all, and it IS
 *   the mechanism React uses to decide a boundary caught something, so calling it directly is a
 *   faithful test of "throw 자식 캐치", not a workaround. It takes `error: unknown` (not `Error`) so
 *   a falsy thrown value (`throw 0`/`throw null`) still flips `state.hasError` to `true` instead of
 *   being mistaken for "no error" — the bug this file's falsy-throw tests below lock down.
 * - `render()` only reads `this.props`/`this.state` and returns a plain React element tree (JSX
 *   produces plain objects — no DOM, no renderer needed) — constructing an instance with `new` and
 *   inspecting what `render()` returns for each `state.hasError` faithfully tests "폴백 렌더" and the
 *   no-error passthrough, without needing anything to actually paint.
 * - `handleRetry` cannot be verified by calling it and re-reading `this.state`, because
 *   `Component.prototype.setState` on an instance that was never mounted by an actual React
 *   renderer has no `updater` attached and silently no-ops (`this.state` would never actually
 *   change) — that is a real limit of testing a class component this way, not something this file
 *   can close. Instead, `setState` is stubbed on the instance and the test asserts `handleRetry`
 *   calls it with the reset value, which is the one thing "재시도 리셋" needs to be true for React's
 *   own (already React-tested) `setState`/re-render machinery to do the rest.
 * - The actual click-driven, full "crash → fallback → click retry → children remount" round trip is
 *   NOT covered here — that needs a live DOM/event system this project's test setup does not
 *   provide. `componentDidCatch` itself, however, only reads its two arguments, calls
 *   `console.error`, and forwards to the optional `onCaught` prop — no React internals involved —
 *   so it IS covered below via a direct call plus a `console.error` spy, the same "call the plain
 *   instance method, no renderer needed" approach the rest of this file already uses.
 * - `componentDidMount`/`componentDidUpdate` (the fallback-focus effect) read `this.fallbackRef.
 *   current`, which stays `null` on an instance built with `new` outside a real React commit — the
 *   same DOM-renderer limit as `handleRetry` above. The tests below only assert the methods don't
 *   throw when the ref is unset; the actual DOM `.focus()` call cannot be verified here.
 */

const echoTranslate = ((key: string) => key) as unknown as TFunction

type ErrorBoundaryBaseProps = ComponentProps<typeof ErrorBoundaryBase>

const buildProps = (children: ErrorBoundaryBaseProps['children']): ErrorBoundaryBaseProps => ({
    labelKey: 'test.regionLabel',
    children,
    t: echoTranslate,
    i18n: {} as unknown as ErrorBoundaryBaseProps['i18n'],
    tReady: true,
})

const asElement = (node: unknown) => node as ReactElement<{ role?: string; children: ReactElement[] }>

describe('ErrorBoundaryBase.getDerivedStateFromError', () => {
    test('던져진 에러를 그대로 state.error 로 반환하고 state.hasError 를 true 로 세운다 (React 가 실제로 호출하는 catch 진입점)', () => {
        const error = new Error('boom')
        expect(ErrorBoundaryBase.getDerivedStateFromError(error)).toEqual({ hasError: true, error })
    })

    test('falsy 값이 던져져도 state.hasError 는 true 가 된다 (throw 0 / throw null 이 "에러 없음"으로 오인되지 않는다)', () => {
        expect(ErrorBoundaryBase.getDerivedStateFromError(0)).toEqual({ hasError: true, error: 0 })
        expect(ErrorBoundaryBase.getDerivedStateFromError(null)).toEqual({ hasError: true, error: null })
    })
})

describe('ErrorBoundaryBase.render', () => {
    test('state.hasError 가 false 면 children 을 그대로 반환한다', () => {
        const children = <div data-testid='child'>ok</div>
        const instance = new ErrorBoundaryBase(buildProps(children))

        expect(instance.render()).toBe(children)
    })

    test('falsy 값이 던져진 뒤에도(state.hasError=true, state.error=0) children 대신 폴백을 렌더한다', () => {
        const children = <div data-testid='child'>ok</div>
        const instance = new ErrorBoundaryBase(buildProps(children))
        instance.state = { hasError: true, error: 0 }

        const fallback = asElement(instance.render())

        expect(fallback).not.toBe(children)
        expect(fallback.type).toBe('div')
        expect(fallback.props.role).toBe('alert')
    })

    test('state.hasError 가 true 면 children 대신 라벨·안내·재시도 버튼을 담은 폴백을 렌더한다', () => {
        const children = <div data-testid='child'>ok</div>
        const instance = new ErrorBoundaryBase(buildProps(children))
        instance.state = { hasError: true, error: new Error('boom') }

        const fallback = asElement(instance.render())

        expect(fallback).not.toBe(children)
        expect(fallback.type).toBe('div')
        expect(fallback.props.role).toBe('alert')

        const [labelSpan, descriptionSpan, ...rest] = fallback.props.children
        expect((labelSpan.props as { children: unknown }).children).toBe('test.regionLabel')
        expect((descriptionSpan.props as { children: unknown }).children).toBe('errorBoundary.description')

        const retryButton = rest[rest.length - 1] as ReactElement<{ children: unknown; onClick: () => void }>
        expect(retryButton.props.children).toBe('common.retry')
        expect(retryButton.props.onClick).toBe(instance.handleRetry)
    })
})

describe('ErrorBoundaryBase.handleRetry', () => {
    test('state 를 hasError:false·error:null 로 되돌리는 setState 호출로 재시도를 구현한다', () => {
        const instance = new ErrorBoundaryBase(buildProps(<div>ok</div>))
        instance.state = { hasError: true, error: new Error('boom') }
        const setStateSpy = mock(() => undefined)
        instance.setState = setStateSpy as unknown as typeof instance.setState

        instance.handleRetry()

        expect(setStateSpy).toHaveBeenCalledWith({ hasError: false, error: null })
    })
})

describe('ErrorBoundaryBase.componentDidCatch', () => {
    test('신규 리포팅 체계 없이 console.error 로만 기록한다 (React 19 기본 콘솔 로깅 유지, 리포팅 도입 금지)', () => {
        const consoleErrorSpy = mock(() => undefined)
        const originalConsoleError = console.error
        console.error = consoleErrorSpy as unknown as typeof console.error

        const error = new Error('boom')
        const errorInfo = { componentStack: '' } as ErrorInfo
        try {
            new ErrorBoundaryBase(buildProps(<div>ok</div>)).componentDidCatch(error, errorInfo)
        } finally {
            console.error = originalConsoleError
        }

        expect(consoleErrorSpy).toHaveBeenCalledWith(error, errorInfo)
    })

    test('onCaught prop 이 있으면 같은 인자로 함께 호출한다 (main.tsx 의 루트 경계 가시화 복구 훅)', () => {
        const onCaughtSpy = mock(() => undefined)
        const instance = new ErrorBoundaryBase({ ...buildProps(<div>ok</div>), onCaught: onCaughtSpy })

        const error = new Error('boom')
        const errorInfo = { componentStack: '' } as ErrorInfo
        const originalConsoleError = console.error
        console.error = mock(() => undefined) as unknown as typeof console.error
        try {
            instance.componentDidCatch(error, errorInfo)
        } finally {
            console.error = originalConsoleError
        }

        expect(onCaughtSpy).toHaveBeenCalledWith(error, errorInfo)
    })

    test('onCaught prop 이 없으면 아무것도 던지지 않는다', () => {
        const instance = new ErrorBoundaryBase(buildProps(<div>ok</div>))
        const originalConsoleError = console.error
        console.error = mock(() => undefined) as unknown as typeof console.error
        try {
            expect(() => instance.componentDidCatch(new Error('boom'), { componentStack: '' } as ErrorInfo)).not.toThrow()
        } finally {
            console.error = originalConsoleError
        }
    })
})

describe('ErrorBoundaryBase.componentDidMount / componentDidUpdate', () => {
    test('fallbackRef 가 아직 DOM 에 붙지 않은 상태(렌더 하네스 없음)에서도 던지지 않는다', () => {
        const instance = new ErrorBoundaryBase(buildProps(<div>ok</div>))
        instance.state = { hasError: true, error: new Error('boom') }

        expect(() => instance.componentDidMount()).not.toThrow()
        expect(() => instance.componentDidUpdate(instance.props, { hasError: false, error: null })).not.toThrow()
    })
})
