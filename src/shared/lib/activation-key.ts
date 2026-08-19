import type { KeyboardEvent } from 'react'

const ACTIVATION_KEYS = new Set(['Enter', ' '])

/**
 * Creates a keydown handler implementing the WAI-ARIA button pattern for
 * elements using `role="button"`: both Enter and Space activate, and Space
 * is prevented from scrolling the page. Ignores events that bubbled up from
 * a nested interactive descendant (its own `role="button"`/`tabIndex` child,
 * e.g. an icon button) so it never double-activates alongside that child.
 */
export const createActivationKeyDownHandler = (onActivate: () => void) => (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget) return
    if (!ACTIVATION_KEYS.has(event.key)) return
    if (event.key === ' ') {
        event.preventDefault()
        if (event.repeat) return
    }
    onActivate()
}
