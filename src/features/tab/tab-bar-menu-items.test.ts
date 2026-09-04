import { describe, expect, test } from 'bun:test'
import { buildTabBarMenuItems } from '@features/tab/tab-bar-menu-items'

const idsOf = (input: Parameters<typeof buildTabBarMenuItems>[0]) => buildTabBarMenuItems(input).map((item) => item.id)

describe('buildTabBarMenuItems — 탭 바 여백 메뉴', () => {
    test('조건이 모두 충족되면 계약 순서대로 전 항목을 낸다', () => {
        expect(idsOf({ surface: 'contextMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: true })).toEqual([
            'newFile',
            'newTerminal',
            'reopenClosed',
            'closeSaved',
            'closeAll',
            'split',
            'openWelcome',
        ])
    })

    test('탭이 없으면 닫기류와 분할을 숨긴다', () => {
        expect(idsOf({ surface: 'contextMenu', hasTabs: false, hasActiveTab: false, hasClosedTabs: true })).toEqual([
            'newFile',
            'newTerminal',
            'reopenClosed',
            'openWelcome',
        ])
    })

    test('활성 탭이 없으면 분할만 숨긴다 — layout_split 이 tabId 를 요구하기 때문', () => {
        expect(idsOf({ surface: 'contextMenu', hasTabs: true, hasActiveTab: false, hasClosedTabs: true })).not.toContain('split')
        expect(idsOf({ surface: 'contextMenu', hasTabs: true, hasActiveTab: false, hasClosedTabs: true })).toContain('closeAll')
    })

    test('닫은 탭이 없으면 다시 열기를 숨긴다', () => {
        expect(idsOf({ surface: 'contextMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: false })).not.toContain('reopenClosed')
    })

    test('조건 인자를 생략하면 전부 false 로 본다', () => {
        expect(idsOf({ surface: 'contextMenu' })).toEqual(['newFile', 'newTerminal', 'openWelcome'])
    })

    test('새 파일·새 터미널·Welcome 은 조건 없이 항상 보인다', () => {
        expect(idsOf({ surface: 'contextMenu' })).toContain('newFile')
        expect(idsOf({ surface: 'contextMenu' })).toContain('newTerminal')
        expect(idsOf({ surface: 'contextMenu' })).toContain('openWelcome')
    })

    test('탭은 있으나 활성 탭·닫은 탭이 없으면 닫기류만 남고 분할·다시 열기는 숨긴다', () => {
        expect(idsOf({ surface: 'contextMenu', hasTabs: true })).toEqual(['newFile', 'newTerminal', 'closeSaved', 'closeAll', 'openWelcome'])
    })

    test('전 항목이 표시될 때 group 은 create → reopen → close → split → welcome 순으로 연속된다', () => {
        const groups = buildTabBarMenuItems({ surface: 'contextMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: true }).map(
            (item) => item.group,
        )
        expect(groups).toEqual(['create', 'create', 'reopen', 'close', 'close', 'split', 'welcome'])
    })

    test('가시성 필터로 그룹이 통째로 빠져도 남은 그룹의 순서는 유지된다', () => {
        const groups = buildTabBarMenuItems({ surface: 'contextMenu', hasTabs: false, hasActiveTab: false, hasClosedTabs: false }).map(
            (item) => item.group,
        )
        expect(groups).toEqual(['create', 'create', 'welcome'])
    })

    test('모든 항목은 라벨 키를 갖고 닫기류 2개만 아이콘이 없다', () => {
        const items = buildTabBarMenuItems({ surface: 'contextMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: true })
        expect(items.every((item) => item.labelKey.length > 0)).toBe(true)
        expect(items.filter((item) => item.icon === null).map((item) => item.id)).toEqual(['closeSaved', 'closeAll'])
    })
})

describe('buildTabBarMenuItems — + 드롭다운', () => {
    test('조건이 모두 충족돼도 생성 항목 2개만 낸다', () => {
        expect(idsOf({ surface: 'addMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: true })).toEqual(['newFile', 'newTerminal'])
    })

    test('조건이 전부 거짓이어도 생성 항목 2개는 그대로 낸다 — + 는 조건 인자를 보지 않는다', () => {
        expect(idsOf({ surface: 'addMenu', hasTabs: false, hasActiveTab: false, hasClosedTabs: false })).toEqual(['newFile', 'newTerminal'])
    })

    test('파괴적 항목(닫기류)은 어떤 조건에서도 + 에 나오지 않는다', () => {
        const ids = idsOf({ surface: 'addMenu', hasTabs: true, hasActiveTab: true, hasClosedTabs: true })
        expect(ids).not.toContain('closeSaved')
        expect(ids).not.toContain('closeAll')
        expect(ids).not.toContain('split')
    })

    test('두 항목 모두 라벨 키와 아이콘을 갖는다', () => {
        const items = buildTabBarMenuItems({ surface: 'addMenu' })
        expect(items.map((item) => item.labelKey)).toEqual(['tab.newUntitledFile', 'tab.newTerminal'])
        expect(items.every((item) => item.icon !== null)).toBe(true)
    })
})
