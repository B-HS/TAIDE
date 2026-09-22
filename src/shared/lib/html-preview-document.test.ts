import { expect, test } from 'bun:test'
import { hierarchicalAssetUrl, htmlPreviewDocument } from '@shared/lib/html-preview-document'

test('로컬과 원격 미리보기 경로에서 상대 이미지와 CSS 폴더를 보존한다', () => {
    const native = hierarchicalAssetUrl('asset://localhost/%2Fproject%20name%2Fpages%2Findex.html', 'tauri://localhost')
    expect(new URL('../styles/main.css', native).href).toBe('asset://localhost//project%20name/styles/main.css')
    const remote = hierarchicalAssetUrl('/__taide/file?path=%2Fproject%20name%2Fpages%2Findex.html', 'https://remote.test')
    expect(new URL('../styles/main.css', remote).href).toBe('https://remote.test/__taide/file//project%20name/styles/main.css')
    const html = new TextEncoder().encode(
        '<html><head><link rel="stylesheet" href="../styles/main.css"></head><body><img src="photo.png"></body></html>',
    )
    const document = new DOMParser().parseFromString(htmlPreviewDocument(html.buffer, remote), 'text/html')
    expect(document.querySelector('base')?.href).toBe('https://remote.test/__taide/file//project%20name/pages/')
    expect(document.querySelector('img')?.getAttribute('src')).toBe('photo.png')
})
