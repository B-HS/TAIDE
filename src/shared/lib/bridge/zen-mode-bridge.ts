import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

type Listener = () => void

const toggleZenModeBridge = createFireAndForgetBridge<undefined>()

export const requestToggleZenMode = () => toggleZenModeBridge.publish(undefined)
export const subscribeToggleZenMode = (listener: Listener) => toggleZenModeBridge.subscribe(() => listener())
