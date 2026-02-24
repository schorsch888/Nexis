import type { ConnectionState } from './messagesStore'
import styles from './ConnectionBadge.module.css'

interface ConnectionBadgeProps {
  state: ConnectionState
}

const stateLabels: Record<ConnectionState, string> = {
  connected: 'Connected',
  connecting: 'Connecting...',
  disconnected: 'Disconnected',
  reconnecting: 'Reconnecting...',
}

const stateClasses: Record<ConnectionState, string> = {
  connected: styles.connected,
  connecting: styles.connecting,
  disconnected: styles.disconnected,
  reconnecting: styles.reconnecting,
}

export function ConnectionBadge({ state }: ConnectionBadgeProps) {
  return (
    <span className={`${styles.badge} ${stateClasses[state]}`}>
      <span className={styles.dot} />
      {stateLabels[state]}
    </span>
  )
}
