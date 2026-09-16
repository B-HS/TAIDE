import { describe, expect, test } from 'bun:test'
import type { ProjectGroupDialogMode } from '@features/project/project-group-dialog'
import { ProjectGroupDialog } from '@features/project/project-group-dialog'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The group name/color form (contract §1.C). Two behaviours carry it, and both are invisible from
 * the outside until something goes wrong:
 *
 * - **Fields are seeded on the closed→open transition, not at mount.** The rail keeps one dialog per
 *   group header mounted and only toggles `open`, so these tests mount closed and then open —
 *   mounting straight into `open` would skip the very transition under test.
 * - **Submit sends the normalized name**, the value `domain::project::service::sanitize_group_name`
 *   would produce, so this side can never compose a name the backend refuses.
 */
type DialogProps = { open: boolean; mode?: ProjectGroupDialogMode; initialName?: string; initialColor?: string | null; isPending?: boolean }

const openDialog = ({ mode = 'create', initialName = '', initialColor = null, isPending = false }: Omit<DialogProps, 'open'> = {}) => {
    const submitted: { name: string; color: string | null }[] = []
    const openChanges: boolean[] = []
    const renderProps = (props: DialogProps) => (
        <ProjectGroupDialog
            open={props.open}
            mode={props.mode ?? mode}
            initialName={props.initialName ?? initialName}
            initialColor={props.initialColor ?? initialColor}
            isPending={props.isPending ?? isPending}
            onOpenChange={(next) => openChanges.push(next)}
            onSubmit={(value) => submitted.push(value)}
        />
    )

    const rendered = renderWithProviders(renderProps({ open: false }))
    const setProps = (props: DialogProps) => rendered.rerender(renderProps(props))
    setProps({ open: true })

    return { ...rendered, setProps, submitted, openChanges }
}

const nameInput = () => screen.getByRole('textbox') as HTMLInputElement

const typeName = (value: string) => fireEvent.change(nameInput(), { target: { value } })

const confirm = (labelKey: string) => fireEvent.click(screen.getByRole('button', { name: labelKey }))

describe('ProjectGroupDialog 제출', () => {
    test('이름을 적고 만들면 정규화된 이름과 색 없음을 넘긴다', () => {
        const { submitted } = openDialog()

        typeName('  Work  ')
        confirm('projectGroup.create')

        expect(submitted).toEqual([{ name: 'Work', color: null }])
    })

    test('스와치를 고르면 그 토큰이 함께 나가고 다시 누르면 해제된다', () => {
        const { submitted } = openDialog()

        typeName('Work')
        fireEvent.click(screen.getByRole('button', { name: 'projectGroup.color 3' }))
        confirm('projectGroup.create')
        fireEvent.click(screen.getByRole('button', { name: 'projectGroup.color 3' }))
        confirm('projectGroup.create')

        expect(submitted).toEqual([
            { name: 'Work', color: 'lane3' },
            { name: 'Work', color: null },
        ])
    })

    test('Enter 로도 제출된다', () => {
        const { submitted } = openDialog()

        typeName('Work')
        fireEvent.keyDown(nameInput(), { key: 'Enter' })

        expect(submitted).toEqual([{ name: 'Work', color: null }])
    })

    test('이름이 공백뿐이면 확인이 비활성이고 제출되지 않는다', () => {
        const { submitted } = openDialog()

        typeName('   ')

        expect((screen.getByRole('button', { name: 'projectGroup.create' }) as HTMLButtonElement).disabled).toBe(true)

        fireEvent.keyDown(nameInput(), { key: 'Enter' })

        expect(submitted).toEqual([])
    })

    test('진행 중에는 제출되지 않는다', () => {
        const { setProps, submitted } = openDialog()

        typeName('Work')
        setProps({ open: true, isPending: true })
        fireEvent.keyDown(nameInput(), { key: 'Enter' })

        expect(submitted).toEqual([])
    })
})

describe('ProjectGroupDialog 편집 모드', () => {
    test('열릴 때 기존 이름·색을 채우고 저장 라벨을 쓴다', () => {
        const { submitted } = openDialog({ mode: 'edit', initialName: 'Work', initialColor: 'lane5' })

        expect(nameInput().value).toBe('Work')
        expect(screen.getByRole('button', { name: 'projectGroup.color 5' }).getAttribute('aria-pressed')).toBe('true')

        confirm('common.save')

        expect(submitted).toEqual([{ name: 'Work', color: 'lane5' }])
    })

    test('취소한 편집은 다음 열기에 남지 않는다 — 닫힘→열림 전이에서 다시 씨앗을 심는다', () => {
        const { setProps } = openDialog({ mode: 'edit', initialName: 'Work', initialColor: null })

        typeName('Throwaway')
        setProps({ open: false, mode: 'edit', initialName: 'Work' })
        setProps({ open: true, mode: 'edit', initialName: 'Work' })

        expect(nameInput().value).toBe('Work')
    })

    test('카탈로그에 없는 색이 저장돼 있으면 아무 스와치도 눌리지 않는다', () => {
        openDialog({ mode: 'edit', initialName: 'Work', initialColor: 'lane99' })

        expect(screen.getAllByRole('button').filter((button) => button.getAttribute('aria-pressed') === 'true')).toEqual([])
    })
})
