import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'
import { shouldForwardNativeNotification, truncateNotificationText } from '@shared/lib/native-notification-gate'
import { NOTIFICATION_TEXT_MAX_CODE_POINTS } from '@shared/constants/notification'
import { isRemoteMirrorRuntime } from '@shared/lib/remote/runtime-environment'
import { getWindowContext } from '@shared/lib/window-context'
import type { NativeNotificationInput } from '@entities/notification/notification.ipc'
import { sendNativeNotification } from '@entities/notification/notification.ipc'

/**
 * Announces that a notification actually reached the OS, so the app can explain itself once per
 * session ("if you didn't see that, notifications are off for TAIDE" — the plugin cannot report a
 * denied permission, `domain::notification::types::NotificationDelivery`). A bridge rather than a
 * module-scope flag because the *sender* is scattered across entities and widgets while the
 * *announcement* belongs to one always-mounted provider that owns the "already said it" state.
 */
const nativeNotificationDeliveredBridge = createFireAndForgetBridge<undefined>()

export const subscribeNativeNotificationDelivered = (listener: () => void) => nativeNotificationDeliveredBridge.subscribe(() => listener())

/**
 * The one entry point every completion event uses to reach the OS notification center.
 *
 * Two questions are answered in two places on purpose. *Should this realm forward at all* is
 * local — every window hears the same backend broadcast, so the duplicate/mirror rule lives here
 * ({@link shouldForwardNativeNotification}) and costs no IPC. *Should the user be interrupted* is
 * app-wide — whether any TAIDE window has focus is invisible from inside one window — so the
 * settings and focus gate stays in Rust and this always pays one IPC to ask it, rather than
 * caching a settings snapshot per window that could disagree with the backend's.
 *
 * `title`/`body` are already translated: Rust owns whether a notification is sent, the frontend
 * owns what it says. Both are capped here ({@link truncateNotificationText}) because several bodies
 * are foreign text of unbounded length — an install script's stderr, a git remote's rejection
 * message — and nothing between here and the notification center shortens them. Failures are
 * swallowed — a notification that could not be sent must never turn into a second failure on top of
 * whatever the user was actually doing.
 */
export const notifyNative = async (input: NativeNotificationInput) => {
    if (!shouldForwardNativeNotification({ windowKind: getWindowContext().kind, isRemoteMirror: isRemoteMirrorRuntime() })) return null
    try {
        const delivery = await sendNativeNotification({
            ...input,
            title: truncateNotificationText(input.title, NOTIFICATION_TEXT_MAX_CODE_POINTS),
            body: truncateNotificationText(input.body, NOTIFICATION_TEXT_MAX_CODE_POINTS),
        })
        if (delivery.outcome === 'delivered') nativeNotificationDeliveredBridge.publish(undefined)
        return delivery
    } catch {
        return null
    }
}
