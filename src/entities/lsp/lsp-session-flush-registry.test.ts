import { describe, expect, test } from 'bun:test'
import type { ProjectId } from '@shared/api/bindings'
import {
    flushAllLspSessions,
    flushLspSessionsForProject,
    registerLspSessionAllFlush,
    registerLspSessionProjectFlush,
} from '@entities/lsp/lsp-session-flush-registry'

const PROJECT_ID = 'project-1' as ProjectId

describe('flushAllLspSessions', () => {
    test('등록된 핸들러를 호출한다', () => {
        let calls = 0
        registerLspSessionAllFlush(() => {
            calls += 1
        })

        flushAllLspSessions()

        expect(calls).toBe(1)
    })

    test('나중에 등록한 핸들러가 이전 핸들러를 대체한다', () => {
        let firstCalls = 0
        let secondCalls = 0
        registerLspSessionAllFlush(() => {
            firstCalls += 1
        })
        registerLspSessionAllFlush(() => {
            secondCalls += 1
        })

        flushAllLspSessions()

        expect(firstCalls).toBe(0)
        expect(secondCalls).toBe(1)
    })
})

describe('flushLspSessionsForProject', () => {
    test('등록 전에 호출해도 예외 없이 무시된다', () => {
        expect(() => flushLspSessionsForProject(PROJECT_ID)).not.toThrow()
    })

    test('등록된 핸들러에 projectId 를 그대로 넘겨 호출한다', () => {
        const calls: ProjectId[] = []
        registerLspSessionProjectFlush((projectId) => {
            calls.push(projectId)
        })

        flushLspSessionsForProject(PROJECT_ID)

        expect(calls).toEqual([PROJECT_ID])
    })
})
