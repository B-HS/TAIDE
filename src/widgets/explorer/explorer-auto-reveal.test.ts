import { describe, expect, test } from 'bun:test'
import { decideAutoReveal } from '@widgets/explorer/explorer-auto-reveal'

const ACTIVE_PATH = '/project/src/main.ts'

const buildInput = (overrides: Partial<Parameters<typeof decideAutoReveal>[0]> = {}) => ({
    enabled: true,
    activePath: ACTIVE_PATH,
    visiblePaths: new Set<string>(),
    sidebarVisible: true,
    explorerViewActive: true,
    lastRevealedPath: null,
    ...overrides,
})

describe('decideAutoReveal', () => {
    test('설정이 꺼져 있으면 skip 한다', () => {
        expect(decideAutoReveal(buildInput({ enabled: false }))).toBe('skip')
    })

    test('활성 파일 경로가 없으면 skip 한다', () => {
        expect(decideAutoReveal(buildInput({ activePath: null }))).toBe('skip')
    })

    test('사이드바가 보이지 않으면(접힘·Zen) skip 한다', () => {
        expect(decideAutoReveal(buildInput({ sidebarVisible: false }))).toBe('skip')
    })

    test('사이드바가 files 뷰가 아니면 skip 한다', () => {
        expect(decideAutoReveal(buildInput({ explorerViewActive: false }))).toBe('skip')
    })

    test('직전에 이미 reveal 한 경로면 skip 한다', () => {
        expect(decideAutoReveal(buildInput({ lastRevealedPath: ACTIVE_PATH }))).toBe('skip')
    })

    test('행이 이미 보이면 IPC 없이 select-only 를 반환한다', () => {
        expect(decideAutoReveal(buildInput({ visiblePaths: new Set([ACTIVE_PATH]) }))).toBe('select-only')
    })

    test('행이 보이지 않으면 reveal-then-select 를 반환한다', () => {
        expect(decideAutoReveal(buildInput({ visiblePaths: new Set(['/project/src/other.ts']) }))).toBe('reveal-then-select')
    })

    test('보이는 행이어도 게이트가 닫혀 있으면 skip 이 우선한다', () => {
        expect(decideAutoReveal(buildInput({ visiblePaths: new Set([ACTIVE_PATH]), sidebarVisible: false }))).toBe('skip')
    })

    test('직전 reveal 경로와 다른 파일로 전환하면 다시 reveal 한다', () => {
        expect(decideAutoReveal(buildInput({ lastRevealedPath: '/project/src/other.ts' }))).toBe('reveal-then-select')
    })

    test('설정이 꺼져 있으면 행이 보이고 직전 reveal 이 없어도 select-only 조차 하지 않는다 (enabled 게이트가 최우선)', () => {
        expect(decideAutoReveal(buildInput({ enabled: false, visiblePaths: new Set([ACTIVE_PATH]), lastRevealedPath: null }))).toBe('skip')
    })

    test('files 뷰가 아니면 행이 보여도 skip 한다', () => {
        expect(decideAutoReveal(buildInput({ explorerViewActive: false, visiblePaths: new Set([ACTIVE_PATH]) }))).toBe('skip')
    })

    test('보이는 행이 하나도 없으면(빈 집합) reveal-then-select 를 반환한다', () => {
        expect(decideAutoReveal(buildInput({ visiblePaths: new Set<string>() }))).toBe('reveal-then-select')
    })

    test('직전 reveal 경로와 같으면 행이 보여도 select-only 를 반복하지 않는다', () => {
        expect(decideAutoReveal(buildInput({ lastRevealedPath: ACTIVE_PATH, visiblePaths: new Set([ACTIVE_PATH]) }))).toBe('skip')
    })
})
