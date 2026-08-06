import { useRef, useState, useSyncExternalStore } from 'react'

type ObjectUrlEntry = { data: ArrayBuffer; mimeType: string; url: string }

const revokeEntry = (entry: ObjectUrlEntry | null) => {
    if (entry) URL.revokeObjectURL(entry.url)
}

export const useObjectUrl = (data: ArrayBuffer | undefined, mimeType: string) => {
    const entryRef = useRef<ObjectUrlEntry | null>(null)

    const [subscribe] = useState(() => (_onStoreChange: () => void) => () => {
        revokeEntry(entryRef.current)
        entryRef.current = null
    })

    const getSnapshot = () => {
        if (!data) return null
        if (entryRef.current?.data === data && entryRef.current.mimeType === mimeType) return entryRef.current.url

        revokeEntry(entryRef.current)
        const url = URL.createObjectURL(new Blob([data], { type: mimeType }))
        entryRef.current = { data, mimeType, url }
        return url
    }

    return useSyncExternalStore(subscribe, getSnapshot)
}
