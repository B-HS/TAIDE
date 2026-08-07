import { SCROLLBAR_MIN_THUMB_PX, SCROLLBAR_SCROLL_EPSILON_PX } from '@shared/constants/scrollbar'

export type ScrollbarGeometry = {
    scrollOffset: number
    scrollSize: number
    clientSize: number
    trackSize: number
}

export type ScrollbarThumbMetrics = {
    scrollable: boolean
    thumbSize: number
    thumbOffset: number
}

export const computeScrollbarThumbMetrics = ({ scrollOffset, scrollSize, clientSize, trackSize }: ScrollbarGeometry): ScrollbarThumbMetrics => {
    const maxScrollOffset = scrollSize - clientSize
    const scrollable = maxScrollOffset > SCROLLBAR_SCROLL_EPSILON_PX && trackSize > 0
    if (!scrollable) return { scrollable: false, thumbSize: trackSize, thumbOffset: 0 }

    const thumbSize = Math.min(trackSize, Math.max(SCROLLBAR_MIN_THUMB_PX, (clientSize / scrollSize) * trackSize))
    const maxThumbOffset = trackSize - thumbSize
    const clampedScrollOffset = Math.min(Math.max(scrollOffset, 0), maxScrollOffset)
    const thumbOffset = maxThumbOffset > 0 ? (clampedScrollOffset / maxScrollOffset) * maxThumbOffset : 0

    return { scrollable: true, thumbSize, thumbOffset }
}

export type ScrollOffsetForThumbDeltaInput = {
    dragStartScrollOffset: number
    deltaPx: number
    scrollSize: number
    clientSize: number
    trackSize: number
    thumbSize: number
}

export const computeScrollOffsetForThumbDelta = ({
    dragStartScrollOffset,
    deltaPx,
    scrollSize,
    clientSize,
    trackSize,
    thumbSize,
}: ScrollOffsetForThumbDeltaInput) => {
    const maxThumbOffset = trackSize - thumbSize
    const maxScrollOffset = scrollSize - clientSize
    if (maxThumbOffset <= 0 || maxScrollOffset <= 0) return dragStartScrollOffset

    const scrollPerThumbPx = maxScrollOffset / maxThumbOffset
    const nextScrollOffset = dragStartScrollOffset + deltaPx * scrollPerThumbPx
    return Math.min(Math.max(nextScrollOffset, 0), maxScrollOffset)
}

export type ScrollOffsetForTrackClickInput = {
    clickOffset: number
    thumbSize: number
    trackSize: number
    scrollSize: number
    clientSize: number
}

export const computeScrollOffsetForTrackClick = ({ clickOffset, thumbSize, trackSize, scrollSize, clientSize }: ScrollOffsetForTrackClickInput) => {
    const maxThumbOffset = trackSize - thumbSize
    const maxScrollOffset = scrollSize - clientSize
    if (maxThumbOffset <= 0 || maxScrollOffset <= 0) return 0

    const targetThumbOffset = Math.min(Math.max(clickOffset - thumbSize / 2, 0), maxThumbOffset)
    return (targetThumbOffset / maxThumbOffset) * maxScrollOffset
}
