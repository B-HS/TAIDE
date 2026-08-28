import type { Hsv } from '@shared/lib/color'
import { hsvToHex } from '@shared/lib/color'

/**
 * Merges a saturation/value pointer sample into `ColorPicker`'s drag-local HSV, pinning the hue to
 * `prev.h` (the hue already held by the in-progress drag) instead of re-reading `baseHue` (the
 * last-committed prop value) on every sample. `prev` is `null` only for the very first sample of a
 * drag (pointerdown, before any local HSV exists), where `baseHue` is the only value available.
 * Without this pin the SV square and hue slider would fight over which one's local state "wins" the
 * hue on each sample — the "두 슬라이더가 같은 로컬 객체를 공유" derivation-consistency requirement in
 * `docs/acknowledge/2026-08-28-d45-theme-preview-flood-contract.md` §4.
 */
export const nextDragHsvFromSquarePointer = (prev: Hsv | null, baseHue: number, s: number, v: number): Hsv => ({
    h: prev?.h ?? baseHue,
    s,
    v,
})

/**
 * Hue-slider counterpart of {@link nextDragHsvFromSquarePointer} — pins the drag-local
 * saturation/value to `prev` instead of re-reading the last-committed prop values, for the same
 * derivation-consistency reason.
 */
export const nextDragHsvFromHuePointer = (prev: Hsv | null, baseSaturation: number, baseValue: number, h: number): Hsv => ({
    h,
    s: prev?.s ?? baseSaturation,
    v: prev?.v ?? baseValue,
})

/**
 * Resolves the single hex value a pointerup (or an up-without-move click) should commit via
 * `onChange` — the only moment `ColorPicker` is allowed to feed the global theme-preview pipeline
 * per `docs/acknowledge/2026-08-28-d45-theme-preview-flood-contract.md` §4: dragging must produce
 * zero re-interpretation/CSS-variable/shiki work, with exactly one commit on release. Returns `null`
 * when there is no drag-local HSV to commit, so a stray pointerup with no matching pointerdown (or a
 * `lostpointercapture` that follows an up that already committed and cleared the local state) is a
 * safe no-op rather than a duplicate commit.
 */
export const commitDragHsv = (dragHsv: Hsv | null): string | null => (dragHsv ? hsvToHex(dragHsv) : null)
