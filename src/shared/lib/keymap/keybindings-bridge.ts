import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

type Listener = () => void

const openKeybindingsEditorBridge = createFireAndForgetBridge<undefined>()

export const requestOpenKeybindingsEditor = () => openKeybindingsEditorBridge.publish(undefined)
export const subscribeOpenKeybindingsEditor = (listener: Listener) => openKeybindingsEditorBridge.subscribe(() => listener())
