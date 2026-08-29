import { describe, expect, test } from 'bun:test'
import { publishFileSaveSettle, subscribeFileSaveSettle } from '@entities/editor/file-save-settle-registry'
import { hasChangedOnDiskConflict, readDraftSafely, shouldSettleDraftAfterDiskWrite, syncModelFromDisk } from '@widgets/editor-pane/editor-draft-sync'

type SyncLog = string[]

/**
 * Reproduces one `EditorPane` commit: the hot-exit mirror restore effect and the disk-content sync
 * effect both queue a microtask, the restore first (its hook is called first in `EditorPane`), and
 * neither can observe the other's `setState` because no re-render happens between the two
 * microtasks. `live.isDirty` stands in for the dirty signal the restore sets synchronously — the
 * render snapshot both effects captured is `false` throughout, which is exactly why a snapshot-based
 * guard clobbers the restored buffer here.
 */
const runCommitWithPendingMirrorRestore = ({ restoresMirror }: { restoresMirror: boolean }) => {
    const log: SyncLog = []
    const live = { isDirty: false }

    if (restoresMirror) {
        queueMicrotask(() => {
            log.push('mirrorRestore')
            live.isDirty = true
        })
    }

    queueMicrotask(() =>
        syncModelFromDisk({
            isDraftDirty: () => live.isDirty,
            hasUnobservedModelEdit: () => false,
            adoptUnobservedModelEdit: () => log.push('adoptUnobservedModelEdit'),
            applyDiskContent: () => log.push('applyDiskContent'),
        }),
    )

    return { log, live }
}

describe('syncModelFromDisk (F1 A1 — 핫엑시트 미러 복원 vs 디스크 sync 순서 역전)', () => {
    test('같은 커밋에서 미러 복원이 먼저 적용되면 디스크 원본으로 되씌우지 않는다', async () => {
        const { log } = runCommitWithPendingMirrorRestore({ restoresMirror: true })

        await Promise.resolve()
        await Promise.resolve()

        expect(log).toEqual(['mirrorRestore'])
    })

    test('복원할 미러가 없으면 평소대로 디스크 내용을 모델에 적용한다', async () => {
        const { log } = runCommitWithPendingMirrorRestore({ restoresMirror: false })

        await Promise.resolve()
        await Promise.resolve()

        expect(log).toEqual(['applyDiskContent'])
    })

    test('dirty 인 pane 은 관측되지 않은 모델 편집 플래그를 소비하지 않는다 (one-shot 보존)', () => {
        let consumed = 0
        syncModelFromDisk({
            isDraftDirty: () => true,
            hasUnobservedModelEdit: () => {
                consumed += 1
                return true
            },
            adoptUnobservedModelEdit: () => undefined,
            applyDiskContent: () => undefined,
        })

        expect(consumed).toBe(0)
    })

    test('clean 인 pane 에 관측되지 않은 편집이 있으면 되씌우지 않고 draft 로 인수한다', () => {
        const log: SyncLog = []
        syncModelFromDisk({
            isDraftDirty: () => false,
            hasUnobservedModelEdit: () => true,
            adoptUnobservedModelEdit: () => log.push('adoptUnobservedModelEdit'),
            applyDiskContent: () => log.push('applyDiskContent'),
        })

        expect(log).toEqual(['adoptUnobservedModelEdit'])
    })
})

/**
 * The bookkeeping every `EditorPane` keeps in React state, reduced to the fields a disk write has to
 * settle: `dirty`, `syncedContent` ("last known disk content"), and whether a hot-exit mirror write
 * is still armed for the draft. Wired to the real `file-save-settle-registry` through the real
 * {@link shouldSettleDraftAfterDiskWrite} guard, so the assertions below exercise the actual
 * subscription contract `use-editor-file-persistence.ts` implements.
 */
const createPaneOnPath = (path: string, { draft, syncedContent }: { draft: string | null; syncedContent: string }) => {
    const pane = { draft, dirty: draft !== null, syncedContent, mirrorArmed: draft !== null }
    const unsubscribe = subscribeFileSaveSettle(path, (content) => {
        if (!shouldSettleDraftAfterDiskWrite(pane.draft, content)) return
        pane.dirty = false
        pane.syncedContent = content
        pane.mirrorArmed = false
    })

    return { pane, unsubscribe }
}

describe('경로 단위 저장 정착 (F1 A7·B7 — 스플릿 동일 파일 / ide_save)', () => {
    test('한쪽 pane 이 저장하면 반대쪽 pane 의 가짜 changed-on-disk 배너와 미러 부활이 함께 사라진다', () => {
        const { pane: left, unsubscribe: unsubscribeLeft } = createPaneOnPath('/repo/split.ts', { draft: 'edited', syncedContent: 'original' })
        const { pane: right, unsubscribe: unsubscribeRight } = createPaneOnPath('/repo/split.ts', { draft: 'edited', syncedContent: 'original' })

        left.dirty = false
        left.syncedContent = 'edited'
        left.mirrorArmed = false

        expect(hasChangedOnDiskConflict({ isDirty: right.dirty, syncedContent: right.syncedContent, diskContent: 'edited' })).toBe(true)
        expect(right.mirrorArmed).toBe(true)

        publishFileSaveSettle('/repo/split.ts', 'edited')

        expect(hasChangedOnDiskConflict({ isDirty: right.dirty, syncedContent: right.syncedContent, diskContent: 'edited' })).toBe(false)
        expect(right.mirrorArmed).toBe(false)
        expect(right.syncedContent).toBe('edited')

        unsubscribeLeft()
        unsubscribeRight()
    })

    test('한 번도 편집되지 않은 pane 도 외부 저장(ide_save) 후 syncedContent 가 최신 디스크 내용으로 정착된다', () => {
        const { pane, unsubscribe } = createPaneOnPath('/repo/external.ts', { draft: null, syncedContent: 'original' })

        publishFileSaveSettle('/repo/external.ts', 'saved-by-claude')

        expect(pane.syncedContent).toBe('saved-by-claude')
        expect(hasChangedOnDiskConflict({ isDirty: pane.dirty, syncedContent: pane.syncedContent, diskContent: 'saved-by-claude' })).toBe(false)

        unsubscribe()
    })

    test('저장 왕복 중 계속 타이핑한 pane 은 정착되지 않고 dirty·미러를 유지한다', () => {
        const { pane, unsubscribe } = createPaneOnPath('/repo/racing.ts', { draft: 'edited', syncedContent: 'original' })
        pane.draft = 'edited-more'

        publishFileSaveSettle('/repo/racing.ts', 'edited')

        expect(pane.dirty).toBe(true)
        expect(pane.mirrorArmed).toBe(true)
        expect(pane.syncedContent).toBe('original')

        unsubscribe()
    })
})

describe('hasChangedOnDiskConflict (F1 A7 — 가짜 배너 판정)', () => {
    test('clean 인 pane 은 디스크가 달라도 배너를 띄우지 않는다 (되읽기로 해소되는 상태)', () => {
        expect(hasChangedOnDiskConflict({ isDirty: false, syncedContent: 'original', diskContent: 'changed' })).toBe(false)
    })

    test('파일 쿼리가 아직 로드되지 않았으면 판정하지 않는다', () => {
        expect(hasChangedOnDiskConflict({ isDirty: true, syncedContent: 'original', diskContent: null })).toBe(false)
    })

    test('syncedContent 가 아직 없으면(탭 전환 직후) 판정하지 않는다', () => {
        expect(hasChangedOnDiskConflict({ isDirty: true, syncedContent: null, diskContent: 'changed' })).toBe(false)
    })

    test('dirty 이고 디스크가 실제로 달라졌을 때만 배너를 띄운다', () => {
        expect(hasChangedOnDiskConflict({ isDirty: true, syncedContent: 'original', diskContent: 'changed' })).toBe(true)
        expect(hasChangedOnDiskConflict({ isDirty: true, syncedContent: 'original', diskContent: 'original' })).toBe(false)
    })
})

describe('shouldSettleDraftAfterDiskWrite (F1 A7 — 경로 단위 저장 정착)', () => {
    test('한 번도 편집되지 않은 pane(draft 없음)도 저장 결과로 정착된다', () => {
        expect(shouldSettleDraftAfterDiskWrite(null, 'saved')).toBe(true)
    })

    test('draft 가 디스크에 쓰인 내용과 같으면 정착된다 (스플릿 반대쪽 pane)', () => {
        expect(shouldSettleDraftAfterDiskWrite('saved', 'saved')).toBe(true)
    })

    test('저장 왕복 중 더 타이핑된 pane 은 dirty 를 유지한다', () => {
        expect(shouldSettleDraftAfterDiskWrite('saved+typed', 'saved')).toBe(false)
    })

    test('빈 문자열 draft 는 null 과 달리 실제 내용으로 비교된다', () => {
        expect(shouldSettleDraftAfterDiskWrite('', '')).toBe(true)
        expect(shouldSettleDraftAfterDiskWrite('', 'saved')).toBe(false)
    })
})

describe('readDraftSafely (폐기된 monaco 모델을 가리키는 draft 리더)', () => {
    test('리더가 없으면 null 이다', () => {
        expect(readDraftSafely(null)).toBeNull()
    })

    test('살아 있는 모델의 리더는 그대로 본문을 돌려준다', () => {
        expect(readDraftSafely(() => 'draft body')).toBe('draft body')
    })

    test('폐기된 모델을 읽는 리더는 throw 대신 null 이다 (탭 닫기·개명 후 flush)', () => {
        const disposedModelReader = () => {
            throw new Error('Model is disposed!')
        }
        expect(readDraftSafely(disposedModelReader)).toBeNull()
    })
})
