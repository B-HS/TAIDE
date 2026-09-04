import { describe, expect, test } from 'bun:test'
import { shouldOpenAnchorExternally } from '@shared/lib/external-anchor'

const PRODUCTION_ORIGIN = 'tauri://localhost'
const DEV_ORIGIN = 'http://localhost:5173'

describe('shouldOpenAnchorExternally', () => {
    test('프로덕션 오리진에서 https 외부 링크는 외부로 연다', () => {
        expect(shouldOpenAnchorExternally('https://example.com/docs', PRODUCTION_ORIGIN)).toBe(true)
    })

    test('프로덕션 오리진에서 http 외부 링크도 외부로 연다', () => {
        expect(shouldOpenAnchorExternally('http://example.com', PRODUCTION_ORIGIN)).toBe(true)
    })

    test('dev 서버 오리진과 같은 오리진이면 앱 안에서 처리한다', () => {
        expect(shouldOpenAnchorExternally('http://localhost:5173/index.html', DEV_ORIGIN)).toBe(false)
    })

    test('호스트가 같아도 포트가 다르면 다른 오리진이므로 외부로 연다', () => {
        expect(shouldOpenAnchorExternally('http://localhost:3000/index.html', DEV_ORIGIN)).toBe(true)
    })

    test('mailto 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('mailto:someone@example.com', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('javascript 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('javascript:alert(1)', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('file 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('file:///etc/passwd', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('blob 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('blob:tauri://localhost/1234', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('앱 자체 스킴(tauri://) 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('tauri://localhost/assets/app.js', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('상대 경로는 절대 URL 로 파싱되지 않으므로 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('/docs/guide', PRODUCTION_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('#anchor', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('빈 href 는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('앱 오리진이 http 일 때 같은 오리진의 http 앵커는 앱 안에서 처리한다 (경로·쿼리·해시가 달라도 오리진 기준)', () => {
        expect(shouldOpenAnchorExternally('http://localhost:5173/docs?x=1#top', DEV_ORIGIN)).toBe(false)
    })

    test('같은 호스트·포트여도 스킴이 다르면(https vs http) 다른 오리진이므로 외부로 연다', () => {
        expect(shouldOpenAnchorExternally('https://localhost:5173/index.html', DEV_ORIGIN)).toBe(true)
    })

    test('스킴·호스트 대소문자는 URL 정규화로 흡수된다 (같은 오리진은 앱 안, 외부는 외부)', () => {
        expect(shouldOpenAnchorExternally('HTTP://LOCALHOST:5173/index.html', DEV_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('HTTPS://EXAMPLE.COM/docs', PRODUCTION_ORIGIN)).toBe(true)
    })

    test('파싱 불가능한 href 는 예외 없이 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('http://', PRODUCTION_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('https://exa mple.com', PRODUCTION_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('not a url', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('상대 href 의 다른 형태(./ · ?query · 프로토콜 상대 //)도 절대 URL 이 아니므로 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('./guide.md', PRODUCTION_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('?page=2', PRODUCTION_ORIGIN)).toBe(false)
        expect(shouldOpenAnchorExternally('//example.com/docs', PRODUCTION_ORIGIN)).toBe(false)
    })

    test('about:blank 링크는 외부로 열지 않는다', () => {
        expect(shouldOpenAnchorExternally('about:blank', PRODUCTION_ORIGIN)).toBe(false)
    })
})
