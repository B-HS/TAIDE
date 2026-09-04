import { describe, expect, test } from 'bun:test'
import type { ProjectDisplayPatch, ProjectRef } from '@shared/api/bindings'
import { ProjectDisplayDialog } from '@features/project/project-display-dialog'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The project display editor (batch 4 contract §D.2-2). Two rules carry the whole dialog and both
 * are invisible from the outside until something goes wrong:
 *
 * - **Save sends all three axes, using `''` for the ones this mode does not use.**
 *   `ProjectDisplayPatch` reads `null` as "leave alone", so a patch that omitted the unused axes
 *   would let a project keep its old icon while displaying a label — and "reset to default" would
 *   clear nothing.
 * - **Every field is seeded on the closed→open transition, not at mount.** The sidebar keeps one
 *   dialog per project row mounted and only toggles `open`, so these tests mount closed and then
 *   open — mounting straight into `open` would skip the very transition under test (and is not how
 *   the dialog is ever used).
 *
 * The glyph itself is a lucide `svg`/text node with no layout under happy-dom, so the CJK clipping
 * and color-contrast checks stay in the hand-QA sheet (`docs/memory/test-conventions.md` §5).
 */
type Display = ProjectRef['display']

type DialogProps = { open: boolean; projectName?: string; display?: Display; isPending?: boolean }

const openDialog = ({ display = undefined, isPending = false, projectName = 'TAIDE' }: Omit<DialogProps, 'open'> = {}) => {
    const submitted: ProjectDisplayPatch[] = []
    const openChanges: boolean[] = []
    const renderProps = (props: DialogProps) => (
        <ProjectDisplayDialog
            open={props.open}
            projectName={props.projectName ?? projectName}
            display={props.display ?? display}
            isPending={props.isPending ?? isPending}
            onOpenChange={(next) => openChanges.push(next)}
            onSubmit={(patch) => submitted.push(patch)}
        />
    )

    const rendered = renderWithProviders(renderProps({ open: false }))
    const setProps = (props: DialogProps) => rendered.rerender(renderProps(props))
    setProps({ open: true })

    return { ...rendered, setProps, submitted, openChanges }
}

const selectMode = (labelKey: string) => fireEvent.click(screen.getByLabelText(labelKey))

const isModeSelected = (labelKey: string) => (screen.getByLabelText(labelKey) as HTMLInputElement).checked

const save = () => fireEvent.click(screen.getByRole('button', { name: 'common.save' }))

const labelInput = () => screen.getByRole('textbox') as HTMLInputElement

describe('ProjectDisplayDialog 저장 패치', () => {
    test('기본 모드는 아이콘·라벨을 빈 문자열로 비워 보낸다 (null 은 "유지" 이므로 초기화가 되지 않는다)', () => {
        const { submitted } = openDialog()

        save()

        expect(submitted).toEqual([{ icon: '', label: '', color: '' }])
    })

    test('아이콘 모드는 고른 아이콘만 담고 라벨은 비운다', () => {
        const { submitted } = openDialog({ display: { icon: 'rocket', label: null, color: null } })

        expect(isModeSelected('project.displayModeIcon')).toBe(true)

        save()

        expect(submitted).toEqual([{ icon: 'rocket', label: '', color: '' }])
    })

    test('아이콘 그리드에서 고른 아이콘이 저장된다', () => {
        const { submitted } = openDialog({ display: { icon: 'rocket', label: null, color: null } })

        fireEvent.click(screen.getByRole('option', { name: 'database' }))
        save()

        expect(submitted).toEqual([{ icon: 'database', label: '', color: '' }])
    })

    test('라벨 모드는 입력한 라벨만 담고 아이콘은 비운다 (세 모드는 배타)', () => {
        const { submitted } = openDialog({ display: { icon: null, label: 'TA', color: null } })

        expect(isModeSelected('project.displayModeLabel')).toBe(true)

        save()

        expect(submitted).toEqual([{ icon: '', label: 'TA', color: '' }])
    })

    test('아이콘이 있던 프로젝트를 라벨 모드로 바꾸면 아이콘 축이 비워진다', () => {
        const { submitted } = openDialog({ display: { icon: 'rocket', label: null, color: null } })

        selectMode('project.displayModeLabel')
        fireEvent.change(labelInput(), { target: { value: 'TA' } })
        save()

        expect(submitted).toEqual([{ icon: '', label: 'TA', color: '' }])
    })

    test('색은 모드와 무관하게 함께 저장된다', () => {
        const { submitted } = openDialog({ display: { icon: null, label: null, color: 'lane3' } })

        save()

        expect(submitted).toEqual([{ icon: '', label: '', color: 'lane3' }])
    })

    test('같은 색 스와치를 다시 누르면 색이 해제된다', () => {
        const { submitted } = openDialog({ display: { icon: null, label: null, color: 'lane3' } })

        fireEvent.click(screen.getByRole('button', { name: 'project.displayColor 3' }))
        save()

        expect(submitted).toEqual([{ icon: '', label: '', color: '' }])
    })

    test('다른 색 스와치를 누르면 그 색으로 바뀐다', () => {
        const { submitted } = openDialog()

        fireEvent.click(screen.getByRole('button', { name: 'project.displayColor 5' }))
        save()

        expect(submitted).toEqual([{ icon: '', label: '', color: 'lane5' }])
    })

    test('라벨은 입력 도중에도 코드포인트 상한으로 잘린다', () => {
        const { submitted } = openDialog({ display: { icon: null, label: 'TA', color: null } })

        fireEvent.change(labelInput(), { target: { value: 'ABCDEFG' } })
        save()

        expect(submitted).toEqual([{ icon: '', label: 'ABCD', color: '' }])
    })

    test('라벨 앞뒤 공백은 저장 시 제거된다', () => {
        const { submitted } = openDialog({ display: { icon: null, label: 'TA', color: null } })

        fireEvent.change(labelInput(), { target: { value: ' A ' } })
        save()

        expect(submitted).toEqual([{ icon: '', label: 'A', color: '' }])
    })

    test('라벨 모드에서 라벨이 비면 저장할 수 없다', () => {
        openDialog({ display: { icon: null, label: 'TA', color: null } })

        fireEvent.change(labelInput(), { target: { value: '   ' } })

        expect(screen.getByRole('button', { name: 'common.save' }).hasAttribute('disabled')).toBe(true)
    })

    test('저장 요청이 진행 중이면 저장 버튼이 잠긴다 (중복 전송 방지)', () => {
        openDialog({ isPending: true })

        expect(screen.getByRole('button', { name: 'common.save' }).hasAttribute('disabled')).toBe(true)
    })

    test('취소는 아무 것도 제출하지 않고 닫기만 요청한다', () => {
        const { submitted, openChanges } = openDialog()

        fireEvent.click(screen.getByRole('button', { name: 'common.cancel' }))

        expect(submitted).toEqual([])
        expect(openChanges).toEqual([false])
    })
})

describe('ProjectDisplayDialog 초기화', () => {
    test('표시 설정이 없으면 기본 모드로 열린다', () => {
        openDialog()

        expect(isModeSelected('project.displayModeDefault')).toBe(true)
        expect(screen.queryByRole('textbox')).toBeNull()
    })

    test('닫았다 다시 열면 취소한 편집이 남지 않는다', () => {
        const display = { icon: null, label: 'TA', color: null }
        const { setProps, submitted } = openDialog({ display })

        selectMode('project.displayModeDefault')
        setProps({ open: false })
        setProps({ open: true })
        save()

        expect(submitted).toEqual([{ icon: '', label: 'TA', color: '' }])
    })

    test('다시 열 때 다른 프로젝트의 설정을 받으면 그 값으로 갱신된다', () => {
        const { setProps, submitted } = openDialog({ display: { icon: null, label: 'TA', color: null } })

        setProps({ open: false })
        setProps({ open: true, projectName: 'Other', display: { icon: 'rocket', label: null, color: 'lane7' } })
        save()

        expect(submitted).toEqual([{ icon: 'rocket', label: '', color: 'lane7' }])
    })
})
