type NumericFieldCommitInput = {
    rawValue: number
    committedValue: number
    min: number
    max: number
}

/**
 * The value a numeric settings field must send when it loses focus, or `null` when it must send
 * nothing: the input is empty (`NaN`), or the clamped input is already what is stored.
 *
 * Pairs with the field re-writing itself to `committedValue` on blur. The input is uncontrolled
 * while being typed in, so on blur the text on screen is whatever the user left there — the old
 * handler clamped the number it *sent* but never touched the input, leaving a field displaying
 * `9999` with a `max` of `100` while the setting was `100`, and leaving a rejected write's typed
 * number standing as if it had been saved (audit §4-B B15). Snapping the field back to the stored
 * value and letting a successful write move it forward keeps what is on screen equal to what is in
 * effect in both outcomes.
 */
export const resolveNumericFieldCommit = ({ rawValue, committedValue, min, max }: NumericFieldCommitInput) => {
    if (Number.isNaN(rawValue)) return null
    const clamped = Math.min(max, Math.max(min, rawValue))
    return clamped === committedValue ? null : clamped
}
