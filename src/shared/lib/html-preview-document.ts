export const htmlPreviewDocument = (data: ArrayBuffer, sourceUrl: string) => {
    const document = new DOMParser().parseFromString(new TextDecoder().decode(data), 'text/html')
    const existingBase = document.querySelector('base[href]')?.getAttribute('href')
    for (const base of document.querySelectorAll('base')) base.remove()
    const base = document.createElement('base')
    base.href = new URL(existingBase ?? '.', sourceUrl).href
    document.head.prepend(base)
    const policy = document.createElement('meta')
    policy.httpEquiv = 'Content-Security-Policy'
    policy.content = "script-src 'none'; object-src 'none'; frame-src 'none'; form-action 'none'"
    document.head.prepend(policy)
    return `<!doctype html>${document.documentElement.outerHTML}`
}

export const hierarchicalAssetUrl = (sourceUrl: string, locationUrl: string) => {
    const url = new URL(sourceUrl, locationUrl)
    const path = url.searchParams.get('path')
    if (url.pathname === '/__taide/file' && path !== null) {
        url.pathname = `/__taide/file/${path.split('/').map(encodeURIComponent).join('/')}`
        url.search = ''
        return url.href
    }
    url.pathname = url.pathname.replace(/%2f/gi, '/')
    return url.href
}
