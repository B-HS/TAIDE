import { describe, expect, test } from 'bun:test'
import { readWindowContext } from '@shared/lib/window-context'

describe('readWindowContext', () => {
    test('쿼리스트링이 없으면 메인 창으로 판단한다', () => {
        expect(readWindowContext('')).toEqual({ kind: 'main' })
    })

    test('projectId·windowSlot 이 모두 있으면 보조 창으로 판단한다', () => {
        expect(readWindowContext('?projectId=prj-1&windowSlot=2')).toEqual({ kind: 'auxiliary', projectId: 'prj-1', windowSlot: 2 })
    })

    test('projectId 만 있고 windowSlot 이 없으면 메인 창으로 판단한다', () => {
        expect(readWindowContext('?projectId=prj-1')).toEqual({ kind: 'main' })
    })

    test('windowSlot 만 있고 projectId 가 없으면 메인 창으로 판단한다', () => {
        expect(readWindowContext('?windowSlot=2')).toEqual({ kind: 'main' })
    })

    test('windowSlot 이 정수가 아니면 메인 창으로 판단한다', () => {
        expect(readWindowContext('?projectId=prj-1&windowSlot=abc')).toEqual({ kind: 'main' })
        expect(readWindowContext('?projectId=prj-1&windowSlot=1.5')).toEqual({ kind: 'main' })
    })

    test('windowSlot=0 처럼 falsy 하지만 유효한 정수는 보조 창으로 판단한다', () => {
        expect(readWindowContext('?projectId=prj-1&windowSlot=0')).toEqual({ kind: 'auxiliary', projectId: 'prj-1', windowSlot: 0 })
    })
})
