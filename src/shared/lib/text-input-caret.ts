/**
 * Minimal structural shape of the text input this helper drives — the value it reads plus the two
 * methods it calls — so the caret rule stays exercisable without a DOM.
 */
type CaretPlaceableTextInput = Pick<HTMLInputElement, 'value' | 'focus' | 'setSelectionRange'>

/**
 * Focuses `input` and collapses the caret to the end of its value, so a programmatic focus never
 * hands the user a fully selected field whose first keystroke wipes what was already there.
 *
 * Two independent sources put that selection there, which is why collapsing happens right after
 * `focus()` rather than by suppressing either one:
 *
 * - radix's `FocusScope` autofocuses the first tabbable candidate through its own
 *   `focus(element, { select: true })` helper, which calls `element.select()` on any text input;
 * - macOS WKWebView reaches the same state unprompted, restoring the default cached selection (the
 *   whole value) whenever a text control is focused programmatically.
 *
 * `preventScroll` matches what radix's helper passes, so replacing its autofocus with this one does
 * not reintroduce a scroll jump toward the field.
 */
export const focusTextInputCaretAtEnd = (input: CaretPlaceableTextInput | null) => {
    if (!input) return
    input.focus({ preventScroll: true })
    input.setSelectionRange(input.value.length, input.value.length)
}
