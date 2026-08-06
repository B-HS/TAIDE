const DARK_MEDIA_QUERY = '(prefers-color-scheme: dark)'

export const readSystemTheme = () => {
    if (typeof window === 'undefined' || !window.matchMedia) return 'dark'
    return window.matchMedia(DARK_MEDIA_QUERY).matches ? 'dark' : 'light'
}

export const subscribeSystemTheme = (onChange: () => void) => {
    if (typeof window === 'undefined' || !window.matchMedia) return () => undefined
    const query = window.matchMedia(DARK_MEDIA_QUERY)
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
}
