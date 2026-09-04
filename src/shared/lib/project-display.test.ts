import { describe, expect, test } from 'bun:test'
import { PROJECT_COLOR_TOKENS, PROJECT_LABEL_CLASS_BY_LENGTH, PROJECT_LABEL_MAX_CODEPOINTS } from '@shared/constants/project-display'
import {
    clampProjectLabel,
    isProjectDisplayCustomized,
    normalizeProjectLabel,
    resolveProjectColorToken,
    resolveProjectDisplay,
    resolveProjectLabelClassName,
} from '@shared/lib/project-display'

describe('resolveProjectDisplay / 기본값', () => {
    test('display 가 없으면 default 모드로 폴백한다', () => {
        expect(resolveProjectDisplay({})).toEqual({ mode: 'default', label: null, icon: null, colorVar: null })
    })

    test('display 의 모든 축이 비어 있으면 default 모드다', () => {
        expect(resolveProjectDisplay({ display: { icon: null, label: null, color: null } }).mode).toBe('default')
        expect(resolveProjectDisplay({ display: { icon: '', label: '   ', color: '' } }).mode).toBe('default')
    })
})

describe('resolveProjectDisplay / 모드 우선순위', () => {
    test('라벨이 있으면 아이콘보다 우선한다', () => {
        const resolution = resolveProjectDisplay({ display: { icon: 'rocket', label: 'TA' } })
        expect(resolution.mode).toBe('label')
        expect(resolution.label).toBe('TA')
        expect(resolution.icon).toBeNull()
    })

    test('라벨이 없고 아이콘만 있으면 icon 모드다', () => {
        const resolution = resolveProjectDisplay({ display: { icon: 'rocket' } })
        expect(resolution.mode).toBe('icon')
        expect(resolution.icon).toBe('rocket')
        expect(resolution.label).toBeNull()
    })

    test('카탈로그에 없는 아이콘 이름도 icon 모드로 그대로 통과시킨다', () => {
        expect(resolveProjectDisplay({ display: { icon: 'no-such-icon' } })).toEqual({
            mode: 'icon',
            label: null,
            icon: 'no-such-icon',
            colorVar: null,
        })
    })

    test('세 축이 모두 있어도 모드는 label 하나뿐이고 아이콘은 버려지되 색은 남는다 — 모드 배타', () => {
        expect(resolveProjectDisplay({ display: { icon: 'rocket', label: 'TA', color: 'lane9' } })).toEqual({
            mode: 'label',
            label: 'TA',
            icon: null,
            colorVar: 'var(--taide-graph-lane9)',
        })
    })

    test('공백뿐인 라벨은 없는 것으로 보고 아이콘 모드로 내려간다', () => {
        const resolution = resolveProjectDisplay({ display: { icon: 'rocket', label: '   ' } })
        expect(resolution.mode).toBe('icon')
        expect(resolution.icon).toBe('rocket')
    })

    test('아이콘 이름의 앞뒤 공백은 제거한다', () => {
        expect(resolveProjectDisplay({ display: { icon: '  rocket  ' } }).icon).toBe('rocket')
    })

    test('라벨은 앞뒤 공백을 제거하고 최대 코드포인트로 자른다', () => {
        expect(resolveProjectDisplay({ display: { label: '  TA  ' } }).label).toBe('TA')
        expect(resolveProjectDisplay({ display: { label: '가나다라마바' } }).label).toBe('가나다라')
    })
})

describe('resolveProjectDisplay / 색', () => {
    test('허용된 토큰은 CSS 변수로 변환한다', () => {
        expect(resolveProjectDisplay({ display: { color: 'lane7' } }).colorVar).toBe('var(--taide-graph-lane7)')
    })

    test('허용되지 않은 색 값은 무시한다', () => {
        expect(resolveProjectDisplay({ display: { color: 'lane13' } }).colorVar).toBeNull()
        expect(resolveProjectDisplay({ display: { color: '#ff0000' } }).colorVar).toBeNull()
    })

    test('토큰은 대소문자를 구분하고 CSS 변수 전체 이름은 토큰이 아니다', () => {
        expect(resolveProjectDisplay({ display: { color: 'LANE1' } }).colorVar).toBeNull()
        expect(resolveProjectDisplay({ display: { color: '--taide-graph-lane1' } }).colorVar).toBeNull()
        expect(resolveProjectDisplay({ display: { color: 'var(--taide-graph-lane1)' } }).colorVar).toBeNull()
    })

    test('미지의 색만 있고 라벨이 있으면 label 모드는 유지하되 색은 없다', () => {
        expect(resolveProjectDisplay({ display: { label: 'TA', color: 'lane99' } })).toEqual({
            mode: 'label',
            label: 'TA',
            icon: null,
            colorVar: null,
        })
    })

    test('색은 모드와 무관하게 유지된다', () => {
        expect(resolveProjectDisplay({ display: { color: 'lane1' } }).mode).toBe('default')
        expect(resolveProjectDisplay({ display: { color: 'lane1' } }).colorVar).toBe('var(--taide-graph-lane1)')
        expect(resolveProjectDisplay({ display: { icon: 'rocket', color: 'lane2' } }).colorVar).toBe('var(--taide-graph-lane2)')
        expect(resolveProjectDisplay({ display: { label: 'TA', color: 'lane3' } }).colorVar).toBe('var(--taide-graph-lane3)')
    })

    test('12개 토큰 전부가 CSS 변수로 해석된다', () => {
        for (const token of PROJECT_COLOR_TOKENS) {
            expect(resolveProjectDisplay({ display: { color: token } }).colorVar).toBe(`var(--taide-graph-${token})`)
        }
    })
})

describe('resolveProjectColorToken', () => {
    test('허용 토큰만 반환하고 나머지는 null 이다', () => {
        expect(resolveProjectColorToken('lane12')).toBe('lane12')
        expect(resolveProjectColorToken(' lane5 ')).toBe('lane5')
        expect(resolveProjectColorToken('lane0')).toBeNull()
        expect(resolveProjectColorToken('')).toBeNull()
        expect(resolveProjectColorToken(null)).toBeNull()
        expect(resolveProjectColorToken(undefined)).toBeNull()
    })
})

describe('isProjectDisplayCustomized', () => {
    test('아무 축도 없으면 커스터마이즈로 보지 않는다', () => {
        expect(isProjectDisplayCustomized(resolveProjectDisplay({}))).toBe(false)
    })

    test('아이콘·라벨·색 중 하나라도 있으면 커스터마이즈다', () => {
        expect(isProjectDisplayCustomized(resolveProjectDisplay({ display: { icon: 'rocket' } }))).toBe(true)
        expect(isProjectDisplayCustomized(resolveProjectDisplay({ display: { label: 'TA' } }))).toBe(true)
        expect(isProjectDisplayCustomized(resolveProjectDisplay({ display: { color: 'lane4' } }))).toBe(true)
    })

    test('렌더 불가능한 색만 저장돼 있으면 커스터마이즈로 보지 않는다', () => {
        expect(isProjectDisplayCustomized(resolveProjectDisplay({ display: { color: 'lane99' } }))).toBe(false)
    })
})

describe('clampProjectLabel', () => {
    test('제어문자를 제거하고 코드포인트 상한까지만 남긴다', () => {
        expect(clampProjectLabel('A\nB\tC')).toBe('ABC')
        expect(clampProjectLabel('가나다라마')).toBe('가나다라')
    })

    test('입력 중 공백은 보존한다', () => {
        expect(clampProjectLabel('A ')).toBe('A ')
        expect(clampProjectLabel('A B')).toBe('A B')
    })

    test('서로게이트 쌍은 코드포인트 1개로 센다', () => {
        expect([...clampProjectLabel('𝐀𝐁𝐂𝐃𝐄')].length).toBe(PROJECT_LABEL_MAX_CODEPOINTS)
    })
})

describe('normalizeProjectLabel', () => {
    test('제어문자 제거 후 trim 하고 상한까지만 남긴다', () => {
        expect(normalizeProjectLabel('  TA  ')).toBe('TA')
        expect(normalizeProjectLabel('\n가나다라마\n')).toBe('가나다라')
        expect(normalizeProjectLabel('   ')).toBe('')
    })
})

describe('resolveProjectLabelClassName', () => {
    test('길이가 길수록 작은 타이포 단계를 고른다', () => {
        expect(resolveProjectLabelClassName('A')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[1])
        expect(resolveProjectLabelClassName('AB')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[2])
        expect(resolveProjectLabelClassName('ABC')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[3])
        expect(resolveProjectLabelClassName('ABCD')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[4])
    })

    test('상한을 넘는 길이는 마지막 단계로 고정된다', () => {
        expect(resolveProjectLabelClassName('ABCDEFG')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[PROJECT_LABEL_MAX_CODEPOINTS])
    })

    test('길이는 코드포인트로 세므로 이모지 4개는 4자 단계다', () => {
        expect(resolveProjectLabelClassName('🚀🚀🚀🚀')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[4])
    })

    test('빈 라벨은 0 인덱스 단계로 떨어진다', () => {
        expect(resolveProjectLabelClassName('')).toBe(PROJECT_LABEL_CLASS_BY_LENGTH[0])
    })
})
