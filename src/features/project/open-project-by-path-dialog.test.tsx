import { describe, expect, test } from 'bun:test'
import { OpenProjectByPathDialog } from '@features/project/open-project-by-path-dialog'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The typed-path project opener (d-58 contract §1.D). Mounted closed and then opened, the way the
 * sidebar keeps it — mounting straight into `open` would skip the closed to open reset these tests
 * exist for.
 */
type DialogProps = { open: boolean; isPending?: boolean }

const openDialog = ({ isPending = false }: Omit<DialogProps, 'open'> = {}) => {
    const confirmed: string[] = []
    const openChanges: boolean[] = []
    const renderProps = (props: DialogProps) => (
        <OpenProjectByPathDialog
            open={props.open}
            isPending={props.isPending ?? isPending}
            onOpenChange={(next) => openChanges.push(next)}
            onConfirm={(path) => confirmed.push(path)}
        />
    )

    const rendered = renderWithProviders(renderProps({ open: false }))
    const setProps = (props: DialogProps) => rendered.rerender(renderProps(props))
    setProps({ open: true })

    return { ...rendered, setProps, confirmed, openChanges }
}

const pathInput = () => screen.getByRole('textbox') as HTMLInputElement

const confirmButton = () => screen.getByRole('button', { name: 'app.openProject' })

describe('OpenProjectByPathDialog 제출', () => {
    test('입력이 비어 있으면 확인 버튼이 비활성이다', () => {
        openDialog()

        expect((confirmButton() as HTMLButtonElement).disabled).toBe(true)
    })

    test('공백만 입력해도 제출되지 않는다', () => {
        const { confirmed } = openDialog()

        fireEvent.change(pathInput(), { target: { value: '   ' } })
        fireEvent.keyDown(pathInput(), { key: 'Enter' })

        expect(confirmed).toEqual([])
    })

    test('확인 버튼은 좌우 공백을 제거한 경로를 넘긴다', () => {
        const { confirmed } = openDialog()

        fireEvent.change(pathInput(), { target: { value: '  ~/repo/alpha  ' } })
        fireEvent.click(confirmButton())

        expect(confirmed).toEqual(['~/repo/alpha'])
    })

    test('Enter 로도 제출된다', () => {
        const { confirmed } = openDialog()

        fireEvent.change(pathInput(), { target: { value: '/repo/beta' } })
        fireEvent.keyDown(pathInput(), { key: 'Enter' })

        expect(confirmed).toEqual(['/repo/beta'])
    })

    test('진행 중이면 Enter 로도 두 번째 제출이 나가지 않는다', () => {
        const { confirmed, setProps } = openDialog()

        fireEvent.change(pathInput(), { target: { value: '/repo/beta' } })
        setProps({ open: true, isPending: true })
        fireEvent.keyDown(pathInput(), { key: 'Enter' })

        expect(confirmed).toEqual([])
    })

    test('취소 버튼은 닫기를 요청한다', () => {
        const { openChanges } = openDialog()

        fireEvent.click(screen.getByRole('button', { name: 'common.cancel' }))

        expect(openChanges).toEqual([false])
    })
})

describe('OpenProjectByPathDialog 재사용', () => {
    test('닫았다가 다시 열면 이전 입력이 남지 않는다 — 취소한 경로가 다음 열기에서 확인 한 번에 열리면 안 된다', () => {
        const { setProps } = openDialog()

        fireEvent.change(pathInput(), { target: { value: '/repo/alpha' } })
        setProps({ open: false })
        setProps({ open: true })

        expect(pathInput().value).toBe('')
    })
})
