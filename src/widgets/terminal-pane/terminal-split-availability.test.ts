import { describe, expect, test } from 'bun:test'
import { MIN_PANEL_SIZE_PX } from '@shared/constants/layout'
import { resolveSplitAvailability, resolveSplitTerminalCwd } from '@widgets/terminal-pane/terminal-split-availability'

const RESIZER_THICKNESS_PX = 1
const REQUIRED_PX = MIN_PANEL_SIZE_PX * 2 + RESIZER_THICKNESS_PX

const metrics = (paneWidthPx: number, paneHeightPx: number, resizerThicknessPx = RESIZER_THICKNESS_PX) => ({
    paneWidthPx,
    paneHeightPx,
    minPaneSizePx: MIN_PANEL_SIZE_PX,
    resizerThicknessPx,
})

describe('resolveSplitAvailability', () => {
    test('필요 폭·높이를 모두 채우면 네 방향이 전부 가능하다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX, REQUIRED_PX))).toEqual({ left: true, right: true, top: true, bottom: true })
    })

    test('경계값 바로 아래 1px 이면 그 축이 막힌다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX - 1, REQUIRED_PX - 1))).toEqual({
            left: false,
            right: false,
            top: false,
            bottom: false,
        })
    })

    test('경계값보다 1px 크면 그 축이 열린다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX + 1, REQUIRED_PX + 1))).toEqual({ left: true, right: true, top: true, bottom: true })
    })

    test('폭만 충분하면 좌우만 가능하다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX, REQUIRED_PX - 1))).toEqual({ left: true, right: true, top: false, bottom: false })
    })

    test('높이만 충분하면 상하만 가능하다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX - 1, REQUIRED_PX))).toEqual({ left: false, right: false, top: true, bottom: true })
    })

    test('구분자 두께 0 이면 최소 크기 2배만으로 충분하다', () => {
        const withoutResizer = MIN_PANEL_SIZE_PX * 2
        expect(resolveSplitAvailability(metrics(withoutResizer, withoutResizer, 0))).toEqual({ left: true, right: true, top: true, bottom: true })
    })

    test('구분자 두께가 두꺼워지면 같은 크기가 막힐 수 있다', () => {
        const withoutResizer = MIN_PANEL_SIZE_PX * 2
        expect(resolveSplitAvailability(metrics(withoutResizer, withoutResizer, 8))).toEqual({
            left: false,
            right: false,
            top: false,
            bottom: false,
        })
    })

    test('측정 실패(0 크기)는 전부 불가로 판정한다', () => {
        expect(resolveSplitAvailability(metrics(0, 0))).toEqual({ left: false, right: false, top: false, bottom: false })
    })

    test('한 축만 0 이면 그 축만 막힌다', () => {
        expect(resolveSplitAvailability(metrics(REQUIRED_PX, 0))).toEqual({ left: true, right: true, top: false, bottom: false })
    })

    test('음수 측정값도 불가로 판정한다', () => {
        expect(resolveSplitAvailability(metrics(-1, -1))).toEqual({ left: false, right: false, top: false, bottom: false })
    })

    test('구분자 두께가 폭·높이 양쪽에 같은 값으로 더해진다', () => {
        const thick = 8
        const requiredWithThick = MIN_PANEL_SIZE_PX * 2 + thick
        expect(resolveSplitAvailability(metrics(requiredWithThick, requiredWithThick - 1, thick))).toEqual({
            left: true,
            right: true,
            top: false,
            bottom: false,
        })
    })

    test('최소 pane 크기가 0 이고 구분자도 0 이면 1px 만 있어도 열린다', () => {
        expect(resolveSplitAvailability({ paneWidthPx: 1, paneHeightPx: 1, minPaneSizePx: 0, resizerThicknessPx: 0 })).toEqual({
            left: true,
            right: true,
            top: true,
            bottom: true,
        })
    })
})

describe('resolveSplitTerminalCwd', () => {
    const projectRoot = '/Users/dev/project'

    test('라이브 cwd 가 루트 안이면 그대로 쓴다', () => {
        const resolved = resolveSplitTerminalCwd({ liveCwd: `${projectRoot}/src`, persistedCwd: projectRoot, tabCwd: null, projectRoot })
        expect(resolved).toBe(`${projectRoot}/src`)
    })

    test('라이브 cwd 가 없으면 세션 cwd 로 폴백한다', () => {
        const resolved = resolveSplitTerminalCwd({ liveCwd: null, persistedCwd: `${projectRoot}/docs`, tabCwd: null, projectRoot })
        expect(resolved).toBe(`${projectRoot}/docs`)
    })

    test('라이브·세션 cwd 가 모두 없으면 탭 cwd 로 폴백한다', () => {
        const resolved = resolveSplitTerminalCwd({ liveCwd: null, persistedCwd: null, tabCwd: `${projectRoot}/tests`, projectRoot })
        expect(resolved).toBe(`${projectRoot}/tests`)
    })

    test('루트 밖 cwd 는 null 로 떨어뜨린다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: '/tmp', persistedCwd: projectRoot, tabCwd: null, projectRoot })).toBeNull()
    })

    test('.. 로 루트를 벗어나는 경로도 null 로 떨어뜨린다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: `${projectRoot}/../other`, persistedCwd: null, tabCwd: null, projectRoot })).toBeNull()
    })

    test('후보 cwd 가 하나도 없으면 null 이다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: null, persistedCwd: null, tabCwd: null, projectRoot })).toBeNull()
    })

    test('프로젝트 루트를 아직 모르면 null 이다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: `${projectRoot}/src`, persistedCwd: null, tabCwd: null, projectRoot: null })).toBeNull()
    })

    test('루트 자기 자신은 허용한다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: projectRoot, persistedCwd: null, tabCwd: null, projectRoot })).toBe(projectRoot)
    })

    test('첫 후보가 루트 밖이면 뒤 후보가 루트 안이어도 넘어가지 않고 null 이다 — 폴백은 부재에만 적용', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: null, persistedCwd: '/tmp', tabCwd: `${projectRoot}/src`, projectRoot })).toBeNull()
    })

    test('루트 이름을 접두사로만 공유하는 형제 디렉터리는 루트 밖이다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: `${projectRoot}-other/src`, persistedCwd: null, tabCwd: null, projectRoot })).toBeNull()
    })

    test('루트 끝의 슬래시는 판정에 영향을 주지 않는다', () => {
        expect(resolveSplitTerminalCwd({ liveCwd: `${projectRoot}/src`, persistedCwd: null, tabCwd: null, projectRoot: `${projectRoot}/` })).toBe(
            `${projectRoot}/src`,
        )
    })
})
