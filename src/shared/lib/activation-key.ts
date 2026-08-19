import type { KeyboardEvent } from 'react'

const ACTIVATION_KEYS = new Set(['Enter', ' '])

/**
 * Creates a keydown handler implementing the WAI-ARIA button pattern for
 * elements using `role="button"`: both Enter and Space activate, and Space
 * is prevented from scrolling the page.
 */
export const createActivationKeyDownHandler = (onActivate: () => void) => (event: KeyboardEvent) => {
    if (!ACTIVATION_KEYS.has(event.key)) return
    if (event.key === ' ') event.preventDefault()
    onActivate()
}
