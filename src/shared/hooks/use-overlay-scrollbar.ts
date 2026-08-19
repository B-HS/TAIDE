import type { RefObject } from 'react'
import { useEffect, useId, useRef } from 'react'
import { SCROLLBAR_AUTO_HIDE_MS } from '@shared/constants/scrollbar'
import {
    computeScrollOffsetForThumbDelta,
    computeScrollOffsetForTrackClick,
    computeScrollPercent,
    computeScrollbarThumbMetrics,
} from '@shared/lib/scrollbar-metrics'

export type OverlayScrollbarOrientation = 'vertical' | 'horizontal'

type UseOverlayScrollbarOptions = {
    viewportRef: RefObject<HTMLDivElement | null>
    orientation?: OverlayScrollbarOrientation
}

type OverlayScrollbarHandles = {
    trackRef: RefObject<HTMLDivElement | null>
    thumbRef: RefObject<HTMLDivElement | null>
}

type DragStart = { pointerOffset: number; scrollOffset: number }

export const useOverlayScrollbar = ({ viewportRef, orientation = 'vertical' }: UseOverlayScrollbarOptions): OverlayScrollbarHandles => {
    const trackRef = useRef<HTMLDivElement>(null)
    const thumbRef = useRef<HTMLDivElement>(null)
    const generatedViewportId = useId()

    useEffect(() => {
        const viewport = viewportRef.current
        const track = trackRef.current
        const thumb = thumbRef.current
        if (!viewport || !track || !thumb) return

        if (!viewport.id) viewport.id = generatedViewportId
        track.setAttribute('aria-controls', viewport.id)

        const isVertical = orientation === 'vertical'
        let scheduledFrame = 0
        let hideTimer: ReturnType<typeof setTimeout> | undefined
        let isDragging = false
        let isHovering = false
        let dragStart: DragStart | null = null

        const readGeometry = () => ({
            scrollOffset: isVertical ? viewport.scrollTop : viewport.scrollLeft,
            scrollSize: isVertical ? viewport.scrollHeight : viewport.scrollWidth,
            clientSize: isVertical ? viewport.clientHeight : viewport.clientWidth,
            trackSize: isVertical ? track.clientHeight : track.clientWidth,
        })

        const measure = () => {
            const geometry = readGeometry()
            const metrics = computeScrollbarThumbMetrics(geometry)
            track.dataset.scrollable = String(metrics.scrollable)
            if (!metrics.scrollable) {
                thumb.style.height = ''
                thumb.style.width = ''
                thumb.style.transform = ''
                track.setAttribute('aria-valuenow', '0')
                return
            }
            track.setAttribute('aria-valuenow', String(computeScrollPercent(geometry)))
            if (isVertical) {
                thumb.style.height = `${metrics.thumbSize}px`
                thumb.style.width = ''
                thumb.style.transform = `translate3d(0, ${metrics.thumbOffset}px, 0)`
            } else {
                thumb.style.width = `${metrics.thumbSize}px`
                thumb.style.height = ''
                thumb.style.transform = `translate3d(${metrics.thumbOffset}px, 0, 0)`
            }
        }

        const scheduleMeasure = () => {
            if (scheduledFrame) return
            scheduledFrame = requestAnimationFrame(() => {
                scheduledFrame = 0
                measure()
            })
        }

        const show = () => {
            track.dataset.visible = 'true'
            if (hideTimer) clearTimeout(hideTimer)
            if (isDragging || isHovering) return
            hideTimer = setTimeout(() => {
                track.dataset.visible = 'false'
            }, SCROLLBAR_AUTO_HIDE_MS)
        }

        const handleScroll = () => {
            scheduleMeasure()
            show()
        }
        viewport.addEventListener('scroll', handleScroll, { passive: true })

        const resizeObserver = new ResizeObserver(scheduleMeasure)
        resizeObserver.observe(viewport)

        const observeContentChildren = () => {
            for (const child of Array.from(viewport.children)) resizeObserver.observe(child)
        }
        observeContentChildren()

        const contentObserver = new MutationObserver(() => {
            observeContentChildren()
            scheduleMeasure()
        })
        contentObserver.observe(viewport, { childList: true })

        const handleTrackPointerEnter = () => {
            isHovering = true
            show()
        }
        const handleTrackPointerLeave = () => {
            isHovering = false
            show()
        }
        track.addEventListener('pointerenter', handleTrackPointerEnter)
        track.addEventListener('pointerleave', handleTrackPointerLeave)

        const handleThumbPointerDown = (event: PointerEvent) => {
            event.preventDefault()
            thumb.setPointerCapture(event.pointerId)
            isDragging = true
            dragStart = {
                pointerOffset: isVertical ? event.clientY : event.clientX,
                scrollOffset: isVertical ? viewport.scrollTop : viewport.scrollLeft,
            }
            show()
        }

        const handleThumbPointerMove = (event: PointerEvent) => {
            if (!dragStart) return
            const pointerOffset = isVertical ? event.clientY : event.clientX
            const deltaPx = pointerOffset - dragStart.pointerOffset
            const geometry = readGeometry()
            const metrics = computeScrollbarThumbMetrics({ ...geometry, scrollOffset: dragStart.scrollOffset })
            const nextScrollOffset = computeScrollOffsetForThumbDelta({
                dragStartScrollOffset: dragStart.scrollOffset,
                deltaPx,
                scrollSize: geometry.scrollSize,
                clientSize: geometry.clientSize,
                trackSize: geometry.trackSize,
                thumbSize: metrics.thumbSize,
            })
            if (isVertical) viewport.scrollTop = nextScrollOffset
            else viewport.scrollLeft = nextScrollOffset
        }

        const endThumbDrag = (event: PointerEvent) => {
            if (thumb.hasPointerCapture(event.pointerId)) thumb.releasePointerCapture(event.pointerId)
            isDragging = false
            dragStart = null
            show()
        }
        thumb.addEventListener('pointerdown', handleThumbPointerDown)
        thumb.addEventListener('pointermove', handleThumbPointerMove)
        thumb.addEventListener('pointerup', endThumbDrag)
        thumb.addEventListener('pointercancel', endThumbDrag)

        const handleTrackPointerDown = (event: PointerEvent) => {
            if (event.target === thumb) return
            const rect = track.getBoundingClientRect()
            const clickOffset = isVertical ? event.clientY - rect.top : event.clientX - rect.left
            const geometry = readGeometry()
            const metrics = computeScrollbarThumbMetrics(geometry)
            const nextScrollOffset = computeScrollOffsetForTrackClick({
                clickOffset,
                thumbSize: metrics.thumbSize,
                trackSize: geometry.trackSize,
                scrollSize: geometry.scrollSize,
                clientSize: geometry.clientSize,
            })
            viewport.scrollTo(isVertical ? { top: nextScrollOffset, behavior: 'smooth' } : { left: nextScrollOffset, behavior: 'smooth' })
        }
        track.addEventListener('pointerdown', handleTrackPointerDown)

        scheduleMeasure()

        return () => {
            viewport.removeEventListener('scroll', handleScroll)
            resizeObserver.disconnect()
            contentObserver.disconnect()
            track.removeEventListener('pointerenter', handleTrackPointerEnter)
            track.removeEventListener('pointerleave', handleTrackPointerLeave)
            thumb.removeEventListener('pointerdown', handleThumbPointerDown)
            thumb.removeEventListener('pointermove', handleThumbPointerMove)
            thumb.removeEventListener('pointerup', endThumbDrag)
            thumb.removeEventListener('pointercancel', endThumbDrag)
            track.removeEventListener('pointerdown', handleTrackPointerDown)
            if (scheduledFrame) cancelAnimationFrame(scheduledFrame)
            if (hideTimer) clearTimeout(hideTimer)
        }
    }, [viewportRef, orientation, generatedViewportId])

    return { trackRef, thumbRef }
}
