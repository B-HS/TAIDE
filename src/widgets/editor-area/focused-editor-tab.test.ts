import { describe, expect, test } from 'bun:test'
import type { Tab } from '@shared/api/bindings'
import { resolveSaveRoutableTabId } from '@widgets/editor-area/focused-editor-tab'

const fileTab: Tab = { id: 'tab-file', kind: { kind: 'file', path: '/repo/a.ts' }, title: 'a.ts' }
const appFileTab: Tab = { id: 'tab-app-file', kind: { kind: 'appFile', target: { kind: 'settings' } }, title: 'settings.json' }
const terminalTab: Tab = { id: 'tab-terminal', kind: { kind: 'terminal', sessionId: 's1' }, title: 'Terminal' }
const untitledTab: Tab = { id: 'tab-untitled', kind: { kind: 'untitled', index: 0 }, title: 'Untitled-1' }

/**
 * Reproduces the d-42 `⌘S` appFile no-op (contract §3, item a): before the fix, this returned
 * `null` for a focused `appFile` tab exactly like it correctly does for `terminal`, so
 * `editor-area.tsx`'s `saveActiveTab` silently no-op'd for `settings.json`/prompt-override tabs
 * with no other save path in the UI.
 */
describe('resolveSaveRoutableTabId', () => {
    test('file 탭이 포커스면 그 탭 id 를 반환한다', () => {
        const leaf = { tabs: [fileTab], active: fileTab.id }
        expect(resolveSaveRoutableTabId(leaf)).toBe(fileTab.id)
    })

    test('appFile 탭이 포커스면 그 탭 id 를 반환한다 (d-42 수정 대상)', () => {
        const leaf = { tabs: [appFileTab], active: appFileTab.id }
        expect(resolveSaveRoutableTabId(leaf)).toBe(appFileTab.id)
    })

    test('terminal 탭이 포커스면 null 이다 (no-op 유지)', () => {
        const leaf = { tabs: [terminalTab], active: terminalTab.id }
        expect(resolveSaveRoutableTabId(leaf)).toBeNull()
    })

    test('untitled 탭이 포커스면 null 이다 (동일 원인의 별건 미수정 결함 — d-42 범위 외로 기록됨)', () => {
        const leaf = { tabs: [untitledTab], active: untitledTab.id }
        expect(resolveSaveRoutableTabId(leaf)).toBeNull()
    })

    test('leaf 가 null 이면 null 이다', () => {
        expect(resolveSaveRoutableTabId(null)).toBeNull()
    })

    test('active 가 가리키는 탭이 tabs 목록에 없으면 null 이다', () => {
        const leaf = { tabs: [fileTab], active: 'missing-tab-id' }
        expect(resolveSaveRoutableTabId(leaf)).toBeNull()
    })
})
