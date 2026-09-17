import { describe, expect, test } from 'bun:test'
import type { PaneNode, Tab } from '@shared/api/bindings'
import { resolveRunTargetTerminalTab, resolveTerminalToggleFallbackTab } from '@widgets/editor-area/terminal-tab-targets'

/**
 * Both helpers answer "which tab in this pane" for the two terminal gestures, and both exist because
 * the plain positional `find` they replaced stopped being right the moment a pane held two terminals
 * — a configuration one context-menu "New Terminal" away.
 */
const terminalTab = (id: string): Tab => ({ id, kind: { kind: 'terminal', sessionId: id }, title: id })
const fileTab = (id: string): Tab => ({ id, kind: { kind: 'file', path: `/repo/${id}.ts` }, title: `${id}.ts` })

const leafOf = (tabs: Tab[], active: string | null): Extract<PaneNode, { node: 'leaf' }> => ({ node: 'leaf', id: 'leaf-1', tabs, active })

describe('resolveRunTargetTerminalTab', () => {
    test('활성 탭이 터미널이면 그 터미널에 쓴다 — 스트립 왼쪽 첫 터미널이 아니다', () => {
        const leaf = leafOf([terminalTab('term-a'), terminalTab('term-b')], 'term-b')

        expect(resolveRunTargetTerminalTab(leaf)?.id).toBe('term-b')
    })

    test('활성 탭이 터미널이 아니면 pane 의 첫 터미널로 돌아간다', () => {
        const leaf = leafOf([terminalTab('term-a'), fileTab('index'), terminalTab('term-b')], 'index')

        expect(resolveRunTargetTerminalTab(leaf)?.id).toBe('term-a')
    })

    test('터미널이 하나뿐이면 활성 여부와 무관하게 그 하나다 — 기존 동작 유지', () => {
        const leaf = leafOf([fileTab('index'), terminalTab('term-a')], 'index')

        expect(resolveRunTargetTerminalTab(leaf)?.id).toBe('term-a')
    })

    test('pane 에 터미널이 없으면 null — 호출부가 새로 연다', () => {
        expect(resolveRunTargetTerminalTab(leafOf([fileTab('index')], 'index'))).toBeNull()
    })
})

describe('resolveTerminalToggleFallbackTab', () => {
    test('터미널이 아닌 첫 탭으로 돌아간다 — 터미널이 둘이어도 그 사이를 맴돌지 않는다', () => {
        const leaf = leafOf([terminalTab('term-a'), fileTab('index'), terminalTab('term-b')], 'term-b')

        expect(resolveTerminalToggleFallbackTab(leaf)?.id).toBe('index')
    })

    test('터미널뿐인 pane 은 null — 아무 일도 하지 않는 편이 터미널끼리 튀는 것보다 낫다', () => {
        const leaf = leafOf([terminalTab('term-a'), terminalTab('term-b')], 'term-b')

        expect(resolveTerminalToggleFallbackTab(leaf)).toBeNull()
    })

    test('활성 탭 자신은 복귀 대상이 될 수 없다', () => {
        const leaf = leafOf([fileTab('index')], 'index')

        expect(resolveTerminalToggleFallbackTab(leaf)).toBeNull()
    })
})
