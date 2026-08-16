import { afterEach, describe, expect, test } from 'bun:test'
import type { KeymapEvent } from '@shared/lib/keymap'
import { APP_KEYMAP } from '@shared/lib/keymap'
import type { KeymapContextGetters } from '@shared/lib/keymap-context'
import {
    armKeymapMonacoDeferral,
    clearKeymapChordPending,
    clearKeymapChordState,
    consumeKeymapMonacoDeferral,
    enterKeymapChordPending,
    getKeymapChordDispatchSnapshot,
    getKeymapChordStoreSnapshot,
    notifyKeymapChordNoMatch,
    resolveKeymapChordPending,
    subscribeKeymapChordNoMatch,
    subscribeKeymapChordStore,
} from '@shared/lib/keymap-chord-store'
import type { KeymapDispatchAction } from '@shared/lib/keymap-dispatch'
import { decideKeymapDispatch } from '@shared/lib/keymap-dispatch'

const MOCK_LISTENER_COUNT = 4
const editorFocusedGetters = { editorTextFocus: () => true, terminalFocus: () => false }
const notEditorGetters = { editorTextFocus: () => false, terminalFocus: () => false }

/**
 * Simulates every mounted `useGlobalKeymap` consumer's window-capture listener independently
 * calling `decideKeymapDispatch` for the *same* physical `KeyboardEvent` — real DOM behavior, since
 * `stopPropagation` never stops sibling listeners on the same target. `onAction` runs *inside* each
 * listener's own turn (immediately after its `decideKeymapDispatch` call, before the next listener's)
 * rather than after the whole batch — real `useGlobalKeymap` callers mutate the store synchronously
 * inside the same handler that just decided (see `use-global-keymap.ts`'s `enter-chord`/
 * `observe-monaco-chord-prefix` cases), so a caller that mutated only after collecting every
 * listener's action would never exercise the store-mutated-mid-fanout race that
 * `getKeymapChordDispatchSnapshot`'s event-identity memoization exists to close.
 */
const dispatchToAllListeners = (event: KeymapEvent, getters: KeymapContextGetters, onAction: (action: KeymapDispatchAction) => void = () => {}) =>
    Array.from({ length: MOCK_LISTENER_COUNT }, () => {
        const action = decideKeymapDispatch(event, APP_KEYMAP, getKeymapChordDispatchSnapshot(event), true, getters)
        onAction(action)
        return action
    })

const SHORT_TIMEOUT_MS = 20

/** The store is a module-level singleton (shared across every `useGlobalKeymap` consumer at runtime) — reset it between tests so assertions don't leak across cases. */
afterEach(() => {
    clearKeymapChordState()
})

describe('enterKeymapChordPending / getKeymapChordStoreSnapshot', () => {
    test('진입하면 pending 상태가 채워진다', () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        const snapshot = getKeymapChordStoreSnapshot()
        expect(snapshot.pending?.entryIds).toEqual(['save'])
        expect(snapshot.pending?.prefix).toEqual({ key: 'k', mods: ['mod'] })
        expect(typeof snapshot.pending?.at).toBe('number')
    })

    test('여러 후보 엔트리를 함께 담을 수 있다(같은 1단을 공유하는 형제 chord)', () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['open-keybindings-editor', 'toggle-zen-mode'])
        expect(getKeymapChordStoreSnapshot().pending?.entryIds).toEqual(['open-keybindings-editor', 'toggle-zen-mode'])
    })

    test('구독자에게 알린다', () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordStore(() => {
            calls += 1
        })
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        expect(calls).toBe(1)
        unsubscribe()
    })

    test(`${SHORT_TIMEOUT_MS}ms 타임아웃이 지나면 자동으로 해제된다`, async () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'], SHORT_TIMEOUT_MS)
        expect(getKeymapChordStoreSnapshot().pending).not.toBeNull()
        await new Promise((resolve) => setTimeout(resolve, SHORT_TIMEOUT_MS * 3))
        expect(getKeymapChordStoreSnapshot().pending).toBeNull()
    })
})

describe('resolveKeymapChordPending', () => {
    test('마이크로태스크로 지연 클리어된다 — 동일 이벤트를 처리하는 다른 리스너가 동기적으로는 여전히 pending 을 본다', async () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        resolveKeymapChordPending()
        expect(getKeymapChordStoreSnapshot().pending).not.toBeNull()
        await Promise.resolve()
        expect(getKeymapChordStoreSnapshot().pending).toBeNull()
    })

    test('같은 틱 안에서 여러 번 호출해도(5개 리스너 팬아웃) 한 번만 해제된다', async () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        resolveKeymapChordPending()
        resolveKeymapChordPending()
        resolveKeymapChordPending()
        await Promise.resolve()
        expect(getKeymapChordStoreSnapshot().pending).toBeNull()
    })
})

describe('clearKeymapChordPending', () => {
    test('즉시(동기) pending 을 해제한다', () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        clearKeymapChordPending()
        expect(getKeymapChordStoreSnapshot().pending).toBeNull()
    })

    test('pending 이 이미 null 이면 구독자에게 알리지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordStore(() => {
            calls += 1
        })
        clearKeymapChordPending()
        expect(calls).toBe(0)
        unsubscribe()
    })
})

describe('resolveKeymapChordPending 은 이미 해제된 상태에서 구독자에게 알리지 않는다', () => {
    test('pending 이 없는 상태에서 호출하면(마이크로태스크 플러시 후) notify 되지 않는다', async () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordStore(() => {
            calls += 1
        })
        resolveKeymapChordPending()
        await Promise.resolve()
        expect(calls).toBe(0)
        unsubscribe()
    })
})

describe('consumeKeymapMonacoDeferral 은 이미 해제된 상태에서 구독자에게 알리지 않는다', () => {
    test('monacoDeferral 이 없는 상태에서 호출하면(마이크로태스크 플러시 후) notify 되지 않는다', async () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordStore(() => {
            calls += 1
        })
        consumeKeymapMonacoDeferral()
        await Promise.resolve()
        expect(calls).toBe(0)
        unsubscribe()
    })
})

describe('armKeymapMonacoDeferral / consumeKeymapMonacoDeferral', () => {
    test('세팅하면 monacoDeferral 이 true 가 된다', () => {
        armKeymapMonacoDeferral()
        expect(getKeymapChordStoreSnapshot().monacoDeferral).toBe(true)
    })

    test('소비도 마이크로태스크로 지연 클리어된다', async () => {
        armKeymapMonacoDeferral()
        consumeKeymapMonacoDeferral()
        expect(getKeymapChordStoreSnapshot().monacoDeferral).toBe(true)
        await Promise.resolve()
        expect(getKeymapChordStoreSnapshot().monacoDeferral).toBe(false)
    })

    test(`${SHORT_TIMEOUT_MS}ms 타임아웃이 지나면 자동으로 해제된다`, async () => {
        armKeymapMonacoDeferral(SHORT_TIMEOUT_MS)
        await new Promise((resolve) => setTimeout(resolve, SHORT_TIMEOUT_MS * 3))
        expect(getKeymapChordStoreSnapshot().monacoDeferral).toBe(false)
    })
})

describe('clearKeymapChordState', () => {
    test('pending 과 monacoDeferral 을 모두 즉시 해제한다', () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        armKeymapMonacoDeferral()
        clearKeymapChordState()
        expect(getKeymapChordStoreSnapshot()).toEqual({ pending: null, monacoDeferral: false })
    })

    test('예약된 지연 클리어(resolve/consume)를 무효화한다 — window blur 가 진행 중인 마이크로태스크를 앞지른다', async () => {
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['save'])
        resolveKeymapChordPending()
        clearKeymapChordState()
        enterKeymapChordPending({ key: 'k', mods: ['mod'] }, ['reopen-closed-tab'])
        await Promise.resolve()
        expect(getKeymapChordStoreSnapshot().pending?.entryIds).toEqual(['reopen-closed-tab'])
    })

    test('이미 idle 상태(pending·monacoDeferral 둘 다 없음)에서 호출하면 구독자에게 알리지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordStore(() => {
            calls += 1
        })
        clearKeymapChordState()
        expect(calls).toBe(0)
        unsubscribe()
    })
})

describe('getKeymapChordDispatchSnapshot', () => {
    test('같은 이벤트 참조로 여러 번 호출하면 그 사이 상태 변이와 무관하게 첫 호출 시점의 스냅샷을 반환한다 (5-리스너 팬아웃 대비)', () => {
        const sharedEvent = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const firstSnapshot = getKeymapChordDispatchSnapshot(sharedEvent)
        expect(firstSnapshot.monacoDeferral).toBe(false)

        armKeymapMonacoDeferral()
        expect(getKeymapChordStoreSnapshot().monacoDeferral).toBe(true)

        const secondSnapshot = getKeymapChordDispatchSnapshot(sharedEvent)
        expect(secondSnapshot.monacoDeferral).toBe(false)
        expect(secondSnapshot).toBe(firstSnapshot)
    })

    test('다른 이벤트 참조로 호출하면 그 사이 반영된 최신 상태를 반환한다 (진짜 다음 keydown)', () => {
        const firstEvent = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        getKeymapChordDispatchSnapshot(firstEvent)
        armKeymapMonacoDeferral()

        const secondEvent = { key: 'b', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        expect(getKeymapChordDispatchSnapshot(secondEvent).monacoDeferral).toBe(true)
    })

    test('clearKeymapChordState 이후에는 같은 이벤트 참조로 다시 불러도 그 사이 클리어된 최신 state 를 읽는다(직전 이벤트 참조를 붙들지 않는다)', () => {
        const event = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        armKeymapMonacoDeferral()
        expect(getKeymapChordDispatchSnapshot(event).monacoDeferral).toBe(true)

        clearKeymapChordState()

        expect(getKeymapChordDispatchSnapshot(event).monacoDeferral).toBe(false)
    })
})

describe('멀티 리스너 팬아웃 통합 — decideKeymapDispatch + 실제 스토어 (Wave H 회귀: monaco chord 유예 창)', () => {
    test('에디터 포커스 상태에서 Cmd+K 를 4개 리스너가 모두 처리해도 전부 observe 로 일관되고, 실제 다음 keydown(Cmd+B)까지 유예가 살아남아 defer-to-monaco 가 된다', async () => {
        const cmdK: KeymapEvent = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const cmdKActions = dispatchToAllListeners(cmdK, editorFocusedGetters, (action) => {
            if (action.type === 'observe-monaco-chord-prefix') armKeymapMonacoDeferral()
        })
        expect(cmdKActions.map((action) => action.type)).toEqual(Array(MOCK_LISTENER_COUNT).fill('observe-monaco-chord-prefix'))

        await Promise.resolve()
        await Promise.resolve()

        const cmdB: KeymapEvent = { key: 'b', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const cmdBActions = dispatchToAllListeners(cmdB, editorFocusedGetters)
        expect(cmdBActions.map((action) => action.type)).toEqual(Array(MOCK_LISTENER_COUNT).fill('defer-to-monaco'))
    })

    test('에디터 포커스가 아닌 상태에서 앱 자체 chord(⌘K ⌘S)의 1단을 4개 리스너가 모두 처리해도 전부 enter-chord 로 일관되고, 실제 2단(⌘S)에서 정확히 매칭된다', async () => {
        const cmdK: KeymapEvent = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const cmdKActions = dispatchToAllListeners(cmdK, notEditorGetters, (action) => {
            if (action.type === 'enter-chord') enterKeymapChordPending(action.prefix, action.entryIds)
        })
        expect(cmdKActions.map((action) => action.type)).toEqual(Array(MOCK_LISTENER_COUNT).fill('enter-chord'))

        await Promise.resolve()
        await Promise.resolve()

        const cmdS: KeymapEvent = { key: 's', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const cmdSActions = dispatchToAllListeners(cmdS, notEditorGetters)
        expect(cmdSActions).toEqual(Array(MOCK_LISTENER_COUNT).fill({ type: 'resolve-chord-matched', entryId: 'open-keybindings-editor' }))
    })

    /**
     * The actual bug this Wave fixed: `open-keybindings-editor` (⌘K ⌘S) and `toggle-zen-mode`
     * (⌘K Z) are two real `APP_KEYMAP` entries sharing the exact same first stage. Before
     * `entryIds` went plural, `findMatchingChordPrefixEntry`'s single-winner `.find()` always
     * picked whichever entry appears first in `APP_KEYMAP`, permanently starving every sibling
     * chord registered after it — this test dispatches the *real* array (not a hand-built fixture)
     * and resolves the second sibling's own second stage (Z) to prove it still fires.
     */
    test('형제 chord(⌘K Z)의 1단도 함께 pending 후보에 담기고, 실제 2단(Z)에서 open-keybindings-editor 가 아니라 toggle-zen-mode 로 정확히 매칭된다', async () => {
        const cmdK: KeymapEvent = { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }
        const cmdKActions = dispatchToAllListeners(cmdK, notEditorGetters, (action) => {
            if (action.type === 'enter-chord') enterKeymapChordPending(action.prefix, action.entryIds)
        })
        expect(cmdKActions.map((action) => action.type)).toEqual(Array(MOCK_LISTENER_COUNT).fill('enter-chord'))
        expect(getKeymapChordStoreSnapshot().pending?.entryIds).toEqual(['open-keybindings-editor', 'toggle-zen-mode'])

        await Promise.resolve()
        await Promise.resolve()

        const z: KeymapEvent = { key: 'z', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }
        const zActions = dispatchToAllListeners(z, notEditorGetters)
        expect(zActions).toEqual(Array(MOCK_LISTENER_COUNT).fill({ type: 'resolve-chord-matched', entryId: 'toggle-zen-mode' }))
    })
})

describe('subscribeKeymapChordNoMatch / notifyKeymapChordNoMatch', () => {
    test('알림을 구독자에게 전달한다', () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordNoMatch(() => {
            calls += 1
        })
        notifyKeymapChordNoMatch()
        expect(calls).toBe(1)
        unsubscribe()
    })

    test('구독 해제 후에는 알림을 받지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeKeymapChordNoMatch(() => {
            calls += 1
        })
        unsubscribe()
        notifyKeymapChordNoMatch()
        expect(calls).toBe(0)
    })
})
