import { describe, expect, mock, test } from 'bun:test'
import type { FC } from 'react'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { ShellSlotFocusProvider, ShellSlotScope, useIsShellSlotFocused } from '@shared/lib/shell-slot-context'
import { renderWithProviders } from '@shared/testing/render'

/**
 * The gate contract §0.1 S-4/U-1 puts in front of every duplicated global listener. `EditorArea` and
 * `TerminalPane` are mounted once per open shell slot and register the *same* `useGlobalKeymap`
 * handlers, so what is locked here is the property the whole feature rests on: one ⌘S reaches the
 * focused slot's handler and no other, and a realm with no slot scope at all (an auxiliary window,
 * any component test) still reaches its handler exactly as before.
 *
 * Driven through the real `useGlobalKeymap` rather than by reading the hook's boolean, because the
 * behaviour that matters is the *keystroke* landing once — `useGlobalKeymap` skips `preventDefault`
 * for an `undefined` handler, which is what leaves the key available to the focused slot instead of
 * being swallowed by whichever slot rendered first.
 *
 * `@shared/lib/monaco/setup` is stubbed the usual way (`docs/memory/test-conventions.md` §3):
 * `keymap-context.ts` reaches monaco at import time to answer `when` clauses.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: { getEditors: () => [] } } }))

const FOCUSED_SLOT_ID = 'shellslot-a'
const OTHER_SLOT_ID = 'shellslot-b'
const PROJECT_ID = 'project-a'

type SaveProbeProps = { name: string; onSave: (name: string) => void }

/** Stands in for `EditorArea`: one component, mounted once per slot, registering the same shortcut. */
const SaveProbe: FC<SaveProbeProps> = ({ name, onSave }) => {
    const isFocused = useIsShellSlotFocused()

    useGlobalKeymap({ save: isFocused ? () => onSave(name) : undefined })

    return null
}

/** ⌘S — `APP_KEYMAP`'s `save` entry, dispatched on `window` where `useKeydownCapture` listens in capture phase. */
const pressSave = () => window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', metaKey: true, bubbles: true, cancelable: true }))

describe('useIsShellSlotFocused 게이팅', () => {
    test('두 슬롯이 같은 단축키를 등록해도 포커스된 슬롯에서만 실행된다', () => {
        const calls: string[] = []
        renderWithProviders(
            <ShellSlotFocusProvider focusedShellSlotId={FOCUSED_SLOT_ID} focusedProjectId={PROJECT_ID}>
                <ShellSlotScope slotId={FOCUSED_SLOT_ID} projectId={PROJECT_ID}>
                    <SaveProbe name='focused' onSave={(name) => calls.push(name)} />
                </ShellSlotScope>
                <ShellSlotScope slotId={OTHER_SLOT_ID} projectId='project-b'>
                    <SaveProbe name='unfocused' onSave={(name) => calls.push(name)} />
                </ShellSlotScope>
            </ShellSlotFocusProvider>,
        )

        pressSave()

        expect(calls).toEqual(['focused'])
    })

    test('슬롯 스코프가 없으면(보조 창·테스트) 항상 실행된다 — 경쟁할 상대가 없다', () => {
        const calls: string[] = []
        renderWithProviders(<SaveProbe name='no-scope' onSave={(name) => calls.push(name)} />)

        pressSave()

        expect(calls).toEqual(['no-scope'])
    })

    test('포커스가 다른 슬롯으로 옮겨가면 그쪽 핸들러로 넘어간다', () => {
        const calls: string[] = []
        const tree = (focused: string) => (
            <ShellSlotFocusProvider focusedShellSlotId={focused} focusedProjectId={PROJECT_ID}>
                <ShellSlotScope slotId={FOCUSED_SLOT_ID} projectId={PROJECT_ID}>
                    <SaveProbe name='a' onSave={(name) => calls.push(name)} />
                </ShellSlotScope>
                <ShellSlotScope slotId={OTHER_SLOT_ID} projectId='project-b'>
                    <SaveProbe name='b' onSave={(name) => calls.push(name)} />
                </ShellSlotScope>
            </ShellSlotFocusProvider>
        )

        const { rerender } = renderWithProviders(tree(FOCUSED_SLOT_ID))
        pressSave()
        rerender(tree(OTHER_SLOT_ID))
        pressSave()

        expect(calls).toEqual(['a', 'b'])
    })
})
