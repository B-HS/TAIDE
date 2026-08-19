import { describe, expect, test } from 'bun:test'
import { captureChangedViewState, hasRestorableModel, parsePersistedViewState } from '@widgets/editor-pane/use-editor-view-state'

describe('parsePersistedViewState', () => {
    test('null·undefined 는 복원할 것이 없으므로 null 이다', () => {
        expect(parsePersistedViewState(null)).toBeNull()
        expect(parsePersistedViewState(undefined)).toBeNull()
    })

    test('빈 문자열도 복원할 것이 없으므로 null 이다', () => {
        expect(parsePersistedViewState('')).toBeNull()
    })

    test('유효한 JSON 은 그대로 파싱해 돌려준다', () => {
        const raw = JSON.stringify({ cursorState: [], viewState: { scrollTop: 42 }, contributionsState: {} })
        expect(parsePersistedViewState(raw)).toEqual(JSON.parse(raw))
    })

    test('손상된 JSON(수동 편집된 layout.json 등)은 throw 대신 null 로 취급한다', () => {
        expect(parsePersistedViewState('{not valid json')).toBeNull()
    })

    test('문법은 유효하지만 최상위가 객체가 아니면(문자열·배열·숫자) null 이다 — layout_set_view_state 는 원격에서도 임의 문자열을 받는다', () => {
        expect(parsePersistedViewState(JSON.stringify('hello'))).toBeNull()
        expect(parsePersistedViewState(JSON.stringify([1, 2, 3]))).toBeNull()
        expect(parsePersistedViewState(JSON.stringify(42))).toBeNull()
    })

    test('cursorState 가 배열이 아니면 null 이다', () => {
        const raw = JSON.stringify({ cursorState: 'not-an-array', viewState: {}, contributionsState: {} })
        expect(parsePersistedViewState(raw)).toBeNull()
    })

    test('viewState 필드가 객체가 아니면(누락 포함) null 이다', () => {
        expect(parsePersistedViewState(JSON.stringify({ cursorState: [], contributionsState: {} }))).toBeNull()
        expect(parsePersistedViewState(JSON.stringify({ cursorState: [], viewState: 'not-an-object', contributionsState: {} }))).toBeNull()
    })
})

const FAKE_VIEW_STATE = {
    cursorState: [],
    viewState: { scrollTop: 7, scrollLeft: 0, firstPosition: { lineNumber: 1, column: 1 }, firstPositionDeltaTop: 0 },
    contributionsState: {},
}

describe('captureChangedViewState (F1 finding 0 — 캐시 미스 탭 전환 시 저장 회수)', () => {
    test('editor 가 없으면(dispose 된 인스턴스를 상태로 아직 들고 있는 경우 포함) null 이다', () => {
        expect(captureChangedViewState(null, 'tab-1', new Map())).toBeNull()
    })

    test('dispose 된 editor 는 saveViewState() 가 null 을 반환하므로 이 함수도 null 이다', () => {
        const disposedEditor = { saveViewState: () => null }
        expect(captureChangedViewState(disposedEditor, 'tab-1', new Map())).toBeNull()
    })

    test('마지막으로 보낸 값과 동일하면(커서를 움직이지 않고 탭만 왕복) 재전송하지 않는다', () => {
        const editor = { saveViewState: () => FAKE_VIEW_STATE }
        const serialized = JSON.stringify(FAKE_VIEW_STATE)
        const lastSent = new Map([['tab-1', serialized]])
        expect(captureChangedViewState(editor, 'tab-1', lastSent)).toBeNull()
    })

    test('마지막으로 보낸 값과 다르면(또는 아직 보낸 적 없으면) 직렬화된 값을 반환한다', () => {
        const editor = { saveViewState: () => FAKE_VIEW_STATE }
        expect(captureChangedViewState(editor, 'tab-1', new Map())).toBe(JSON.stringify(FAKE_VIEW_STATE))
    })

    test('같은 값이라도 다른 tabId 기준으로는 아직 보낸 적 없는 값으로 취급한다', () => {
        const editor = { saveViewState: () => FAKE_VIEW_STATE }
        const serialized = JSON.stringify(FAKE_VIEW_STATE)
        const lastSent = new Map([['tab-1', serialized]])
        expect(captureChangedViewState(editor, 'tab-2', lastSent)).toBe(serialized)
    })
})

describe('viewState 저장→복원 왕복 (계약 §3 완료 조건)', () => {
    test('captureChangedViewState 로 직렬화한 값을 parsePersistedViewState 로 되돌리면 원본과 같다', () => {
        const editor = { saveViewState: () => FAKE_VIEW_STATE }
        const serialized = captureChangedViewState(editor, 'tab-1', new Map())
        expect(serialized).not.toBeNull()
        expect(parsePersistedViewState(serialized)).toEqual(FAKE_VIEW_STATE)
    })

    test('저장할 것이 없다고 판단된 경우(dedup·dispose)는 애초에 복원 시도로 이어지지 않는다', () => {
        const serialized = JSON.stringify(FAKE_VIEW_STATE)
        const lastSent = new Map([['tab-1', serialized]])
        const editor = { saveViewState: () => FAKE_VIEW_STATE }
        expect(captureChangedViewState(editor, 'tab-1', lastSent)).toBeNull()
    })
})

describe('hasRestorableModel (F1 finding 1 — no-op 복원에서 가드가 소모되지 않도록)', () => {
    test('모델이 붙어 있으면(정상 마운트·모델 스왑 완료) true 다', () => {
        expect(hasRestorableModel({ getModel: () => ({}) as never })).toBe(true)
    })

    test('dispose 된(또는 아직 모델이 붙지 않은) editor 는 getModel() 이 null 이라 false 다', () => {
        expect(hasRestorableModel({ getModel: () => null })).toBe(false)
    })
})
