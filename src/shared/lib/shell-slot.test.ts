import { describe, expect, test } from 'bun:test'
import type { ShellSlotTree } from '@shared/api/bindings'
import {
    SHELL_SLOT_ID_ATTRIBUTE,
    projectOfShellSlot,
    resolveShellSlotFocus,
    resolveShellSlotIdFromEventTarget,
    shellSlotLeaves,
    shellSlotOfProject,
    withShellSlotToggled,
} from '@shared/lib/shell-slot'

/**
 * The view half of the shell-slot tree (contract §1.B). Rust owns the tree and its mutations
 * (`domain::project::shell_slots`, tested there); what is left on this side is reading it — paint
 * order, the two id↔project lookups the window chrome runs on, and the focus fallback that keeps a
 * stale slot id from leaving the window with nothing focused.
 */
const leaf = (slotId: string, projectId: string): ShellSlotTree => ({ node: 'leaf', slotId, projectId })

const LEFT = leaf('shellslot-a', 'project-a')
const TOP_RIGHT = leaf('shellslot-b', 'project-b')
const BOTTOM_RIGHT = leaf('shellslot-c', 'project-c')

/** Left pane, then a right pane split in two — the arrangement two "Open to the Right"/"Open Below" drops produce. */
const NESTED: ShellSlotTree = {
    node: 'split',
    dir: 'horizontal',
    sizes: [50, 50],
    children: [LEFT, { node: 'split', dir: 'vertical', sizes: [50, 50], children: [TOP_RIGHT, BOTTOM_RIGHT] }],
}

describe('shellSlotLeaves', () => {
    test('단일 리프 트리는 슬롯 1개로 읽힌다 — 분할 이전 화면과 같은 상태', () => {
        expect(shellSlotLeaves(LEFT)).toEqual([{ slotId: 'shellslot-a', projectId: 'project-a' }])
    })

    test('중첩 분할은 화면에 그려지는 순서(왼→오, 위→아래)로 평탄화된다', () => {
        expect(shellSlotLeaves(NESTED).map((entry) => entry.slotId)).toEqual(['shellslot-a', 'shellslot-b', 'shellslot-c'])
    })

    test('트리가 없으면(프로젝트 0개) 빈 목록이다', () => {
        expect(shellSlotLeaves(null)).toEqual([])
    })
})

describe('슬롯 ↔ 프로젝트 조회', () => {
    test('슬롯 id 로 그 슬롯의 프로젝트를 찾는다', () => {
        expect(projectOfShellSlot(NESTED, 'shellslot-c')).toBe('project-c')
    })

    test('프로젝트로 그 프로젝트가 들어 있는 슬롯을 찾는다', () => {
        expect(shellSlotOfProject(NESTED, 'project-b')).toBe('shellslot-b')
    })

    test('트리에 없는 값이면 null 이다', () => {
        expect(projectOfShellSlot(NESTED, 'shellslot-gone')).toBeNull()
        expect(shellSlotOfProject(NESTED, null)).toBeNull()
    })
})

describe('resolveShellSlotFocus', () => {
    test('요청한 슬롯이 트리에 있으면 그대로 쓴다', () => {
        expect(resolveShellSlotFocus(NESTED, 'shellslot-b')).toBe('shellslot-b')
    })

    test('요청한 슬롯이 사라졌으면 첫 슬롯으로 물러난다 — 슬롯 id 는 배치 안의 주소라 재부팅·닫기로 낡을 수 있다', () => {
        expect(resolveShellSlotFocus(NESTED, 'shellslot-closed')).toBe('shellslot-a')
        expect(resolveShellSlotFocus(NESTED, null)).toBe('shellslot-a')
    })

    test('슬롯이 하나도 없으면 null 이다', () => {
        expect(resolveShellSlotFocus(null, 'shellslot-a')).toBeNull()
    })
})

describe('resolveShellSlotIdFromEventTarget', () => {
    test('슬롯 안의 요소는 가장 가까운 슬롯 id 로 해소된다', () => {
        const slot = document.createElement('div')
        slot.setAttribute(SHELL_SLOT_ID_ATTRIBUTE, 'shellslot-a')
        const inner = document.createElement('button')
        slot.appendChild(inner)

        expect(resolveShellSlotIdFromEventTarget(inner)).toBe('shellslot-a')
    })

    test('슬롯 밖(사이드바·상태바·포털)은 null — 포커스를 옮기지 않는다는 뜻이다', () => {
        expect(resolveShellSlotIdFromEventTarget(document.createElement('div'))).toBeNull()
        expect(resolveShellSlotIdFromEventTarget(null)).toBeNull()
        expect(resolveShellSlotIdFromEventTarget(window)).toBeNull()
    })
})

describe('withShellSlotToggled', () => {
    test('Problems 는 슬롯별로 켜지고 꺼진다 — 다른 슬롯의 상태는 건드리지 않는다', () => {
        const opened = withShellSlotToggled([], 'shellslot-a')
        const both = withShellSlotToggled(opened, 'shellslot-b')

        expect(both).toEqual(['shellslot-a', 'shellslot-b'])
        expect(withShellSlotToggled(both, 'shellslot-a')).toEqual(['shellslot-b'])
    })

    test('원본 배열을 바꾸지 않는다', () => {
        const original = ['shellslot-a']
        withShellSlotToggled(original, 'shellslot-b')

        expect(original).toEqual(['shellslot-a'])
    })
})
