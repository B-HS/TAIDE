import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

const revealInExplorerBridge = createFireAndForgetBridge<string>()

export const requestRevealInExplorer = revealInExplorerBridge.publish
export const subscribeRevealInExplorer = revealInExplorerBridge.subscribe
