import type { ConnectionState as ConnectionStatus } from '../../shared/ws/types'
import styles from './ConnectionState.module.css'

interface ConnectionStateProps {
  state: ConnectionStatus
  unreadCount: number
}

const stateLabels: Record<ConnectionStatus, string> = {
  connected: 'Connected',
  connecting: 'Connecting...',
  disconnected: 'Disconnected',
  reconnecting: 'Reconnecting...',
}

const stateClasses: Record<ConnectionStatus, string> = {
  connected: styles.connected,
  connecting: styles.connecting,
  disconnected: styles.disconnected,
  reconnecting: styles.reconnecting,
}

export function ConnectionState({ state, unreadCount }: ConnectionStateProps) {
  return (
    <div className={styles.container}>
      <span className={`${styles.badge} ${stateClasses[state]}`}>
        <span className={styles.dot} />
        {stateLabels[state]}
      </span>
      {unreadCount > 0 && <span className={styles.unread}>{unreadCount} unread</span>}
    </div>
  )
}
