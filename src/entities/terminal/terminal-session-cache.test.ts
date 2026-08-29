import { describe, expect, test } from 'bun:test'
import type { TerminalSession } from '@shared/api/bindings'
import {
    isTerminalSessionAlive,
    markTerminalSessionExited,
    removeTerminalSession,
    upsertTerminalSession,
} from '@entities/terminal/terminal-session-cache'

const PROJECT_ID = 'project-1'

const session = (id: string, running = true): TerminalSession => ({ id, projectId: PROJECT_ID, cwd: '/repo', shell: 'default', running })

describe('upsertTerminalSession', () => {
    test('스폰된 세션을 목록에 추가한다 — 탭 복귀 시 재부착 판정의 근거', () => {
        expect(upsertTerminalSession([], session('a'))).toEqual([session('a')])
    })

    test('같은 id 가 이미 있으면 교체한다', () => {
        const replaced = upsertTerminalSession([session('a', false), session('b')], session('a'))
        expect(replaced).toEqual([session('a'), session('b')])
    })

    test('미조회 캐시(undefined)에는 쓰지 않는다 — 부분 목록이 전체 진실로 굳는 것을 방지', () => {
        expect(upsertTerminalSession(undefined, session('a'))).toBeUndefined()
    })
})

describe('markTerminalSessionExited', () => {
    test('해당 세션만 running=false 로 내린다', () => {
        expect(markTerminalSessionExited([session('a'), session('b')], 'a')).toEqual([session('a', false), session('b')])
    })

    test('목록에 없는 세션이면 변경 없음(undefined)을 반환한다', () => {
        expect(markTerminalSessionExited([session('a')], 'zzz')).toBeUndefined()
    })

    test('이미 running=false 이면 변경 없음(undefined)을 반환한다 — 불필요한 리렌더 차단', () => {
        expect(markTerminalSessionExited([session('a', false)], 'a')).toBeUndefined()
    })

    test('미조회 캐시(undefined)는 그대로 둔다', () => {
        expect(markTerminalSessionExited(undefined, 'a')).toBeUndefined()
    })
})

describe('isTerminalSessionAlive', () => {
    test('스폰을 캐시에 반영하지 않으면 죽은 것으로 읽혀 새 셸을 스폰하게 된다 — A6 재현', () => {
        const rosterFetchedBeforeSpawn: TerminalSession[] = []
        expect(isTerminalSessionAlive(rosterFetchedBeforeSpawn, 'spawned-1')).toBe(false)
    })

    test('스폰을 upsert 로 반영하면 재부착 대상으로 읽힌다 — A6 수정 후', () => {
        const roster = upsertTerminalSession([], session('spawned-1'))
        expect(isTerminalSessionAlive(roster, 'spawned-1')).toBe(true)
    })

    test('exited 로 내려간 세션은 재부착 대상이 아니다 — B14 죽은 세션 attach 차단', () => {
        const roster = markTerminalSessionExited([session('a')], 'a')
        expect(isTerminalSessionAlive(roster, 'a')).toBe(false)
    })

    test('미조회 캐시(undefined)는 살아 있다고 보지 않는다', () => {
        expect(isTerminalSessionAlive(undefined, 'a')).toBe(false)
    })
})

describe('removeTerminalSession', () => {
    test('kill 된 세션을 목록에서 뺀다', () => {
        expect(removeTerminalSession([session('a'), session('b')], 'a')).toEqual([session('b')])
    })

    test('없는 세션이면 변경 없음(undefined)을 반환한다', () => {
        expect(removeTerminalSession([session('a')], 'zzz')).toBeUndefined()
    })

    test('미조회 캐시(undefined)는 그대로 둔다', () => {
        expect(removeTerminalSession(undefined, 'a')).toBeUndefined()
    })
})
