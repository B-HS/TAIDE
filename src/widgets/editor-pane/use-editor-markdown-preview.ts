import { useEffect, useRef, useState } from 'react'

/**
 * Owns `EditorPane`'s markdown preview pane state — the toggle-visible flag, the debounced
 * preview source, and the timer used to arm it. `previewTimeoutRef`/`setPreviewSource` are
 * returned raw (not wrapped) so `use-editor-file-persistence.ts`'s `handleChange` can arm the
 * same debounce timer inline, exactly as it did before this file existed.
 */
export const useEditorMarkdownPreview = () => {
    const previewTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

    const [showMarkdownPreview, setShowMarkdownPreview] = useState(false)
    const [previewSource, setPreviewSource] = useState<string | null>(null)

    useEffect(() => () => clearTimeout(previewTimeoutRef.current), [])

    return { previewTimeoutRef, showMarkdownPreview, setShowMarkdownPreview, previewSource, setPreviewSource }
}
