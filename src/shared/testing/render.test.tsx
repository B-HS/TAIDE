import { describe, expect, test } from 'bun:test'
import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { act, fireEvent, renderHookWithProviders, renderWithProviders, screen } from '@shared/testing/render'

/**
 * Proves the DOM harness itself, not any product code: that `useEffect` runs and cleans up, that a
 * click reaches an `onClick` handler and re-renders, that a `ref` points at a real focusable node,
 * and that both providers `renderWithProviders` installs are reachable from inside the tree. These
 * four are exactly what `react-dom/server` could not do and why happy-dom was added — if this file
 * fails, no component or hook test in the repository can be trusted.
 */
const SMOKE_QUERY_KEY = ['shared', 'testing', 'smoke']
const SMOKE_QUERY_VALUE = 'loaded'

type SmokeCounterProps = { onMount: () => void; onUnmount: () => void }

const SmokeCounter: FC<SmokeCounterProps> = ({ onMount, onUnmount }) => {
    const inputRef = useRef<HTMLInputElement>(null)
    const [count, setCount] = useState(0)
    const { t } = useTranslation()

    useEffect(() => {
        onMount()
        inputRef.current?.focus()
        return onUnmount
    }, [onMount, onUnmount])

    return (
        <div>
            <input ref={inputRef} aria-label='smoke input' />
            <button type='button' onClick={() => setCount(count + 1)}>
                {t('smoke.increment')}
            </button>
            <span role='status'>{count}</span>
        </div>
    )
}

const SmokeQueryValue: FC = () => {
    const { data } = useQuery({ queryKey: SMOKE_QUERY_KEY, queryFn: () => Promise.resolve(SMOKE_QUERY_VALUE) })

    return <span role='status'>{data ?? 'pending'}</span>
}

const useSmokeCounter = () => {
    const [count, setCount] = useState(0)

    return { count, increment: () => setCount((previous) => previous + 1) }
}

const noop = () => undefined

describe('renderWithProviders', () => {
    test('useEffect 가 마운트 시 실행되고 언마운트 시 정리 함수를 호출한다', () => {
        const calls: string[] = []
        const { unmount } = renderWithProviders(<SmokeCounter onMount={() => calls.push('mount')} onUnmount={() => calls.push('unmount')} />)

        expect(calls).toEqual(['mount'])

        unmount()

        expect(calls).toEqual(['mount', 'unmount'])
    })

    test('ref 가 실제 DOM 노드를 가리켜 effect 안의 focus() 가 활성 요소를 바꾼다', () => {
        renderWithProviders(<SmokeCounter onMount={noop} onUnmount={noop} />)

        expect(document.activeElement).toBe(screen.getByLabelText('smoke input'))
    })

    test('클릭 이벤트가 핸들러를 거쳐 상태를 바꾸고 화면을 다시 그린다', () => {
        renderWithProviders(<SmokeCounter onMount={noop} onUnmount={noop} />)

        expect(screen.getByRole('status').textContent).toBe('0')

        fireEvent.click(screen.getByRole('button'))

        expect(screen.getByRole('status').textContent).toBe('1')
    })

    test('i18n provider 가 붙어 있어 번역 키가 그대로 렌더된다 (리소스 번들 없음)', () => {
        renderWithProviders(<SmokeCounter onMount={noop} onUnmount={noop} />)

        expect(screen.getByRole('button').textContent).toBe('smoke.increment')
    })

    test('앞 테스트가 마운트한 트리는 afterEach(cleanup) 로 이미 걷혀 있다', () => {
        expect(document.body.childElementCount).toBe(0)

        renderWithProviders(<SmokeCounter onMount={noop} onUnmount={noop} />)

        expect(document.body.childElementCount).toBe(1)
    })

    test('QueryClientProvider 가 붙어 있어 useQuery 가 해소되면 데이터를 렌더한다', async () => {
        renderWithProviders(<SmokeQueryValue />)

        expect(screen.getByRole('status').textContent).toBe('pending')

        await screen.findByText(SMOKE_QUERY_VALUE)

        expect(screen.getByRole('status').textContent).toBe(SMOKE_QUERY_VALUE)
    })
})

describe('renderHookWithProviders', () => {
    test('훅 단독 실행에서 act 로 감싼 상태 갱신이 반영된다', () => {
        const { result } = renderHookWithProviders(() => useSmokeCounter())

        expect(result.current.count).toBe(0)

        act(() => result.current.increment())

        expect(result.current.count).toBe(1)
    })

    test('돌려받은 queryClient 가 훅이 컨텍스트에서 읽는 인스턴스와 같다', () => {
        const { result, queryClient } = renderHookWithProviders(() => useQueryClient())

        expect(result.current).toBe(queryClient)
    })
})
