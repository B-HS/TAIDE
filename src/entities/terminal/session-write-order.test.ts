import { describe, expect, test } from 'bun:test'
import { enqueueSessionWrite } from '@entities/terminal/session-write-order'

const deferred = () => {
    let resolve: (value: string) => void = () => undefined
    let reject: (error: Error) => void = () => undefined
    const promise = new Promise<string>((resolveFn, rejectFn) => {
        resolve = resolveFn
        reject = rejectFn
    })
    return { promise, resolve, reject }
}

describe('enqueueSessionWrite', () => {
    test('같은 세션의 쓰기는 앞선 쓰기가 끝난 뒤에 시작한다', async () => {
        const started: string[] = []
        const first = deferred()

        const firstWrite = enqueueSessionWrite('session-a', () => {
            started.push('first')
            return first.promise
        })
        const secondWrite = enqueueSessionWrite('session-a', () => {
            started.push('second')
            return Promise.resolve('second')
        })

        await Promise.resolve()
        expect(started).toEqual(['first'])

        first.resolve('first')
        await Promise.all([firstWrite, secondWrite])

        expect(started).toEqual(['first', 'second'])
    })

    test('다른 세션의 쓰기는 서로를 기다리지 않는다', async () => {
        const started: string[] = []
        const blocked = deferred()

        const blockedWrite = enqueueSessionWrite('session-b', () => {
            started.push('b')
            return blocked.promise
        })
        await enqueueSessionWrite('session-c', () => {
            started.push('c')
            return Promise.resolve('c')
        })

        expect(started).toEqual(['b', 'c'])

        blocked.resolve('b')
        await blockedWrite
    })

    test('실패한 쓰기가 뒤따르는 쓰기를 막지 않는다', async () => {
        const started: string[] = []
        const failing = deferred()

        const failingWrite = enqueueSessionWrite('session-d', () => {
            started.push('failing')
            return failing.promise
        })
        const nextWrite = enqueueSessionWrite('session-d', () => {
            started.push('next')
            return Promise.resolve('next')
        })

        failing.reject(new Error('pty gone'))
        await expect(failingWrite).rejects.toThrow('pty gone')

        expect(await nextWrite).toBe('next')
        expect(started).toEqual(['failing', 'next'])
    })
})
