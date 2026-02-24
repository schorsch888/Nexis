import { useAuthStore } from '../../features/auth/authStore'
import { TenantSwitcher } from '../../features/tenant/TenantSwitcher'
import styles from './TopBar.module.css'

export function TopBar() {
  const { logout, user } = useAuthStore()

  return (
    <header className={styles.topbar}>
      <div className={styles.brand}>
        <h1>Nexis</h1>
      </div>
      <div className={styles.actions}>
        <TenantSwitcher />
        {user && (
          <div className={styles.user}>
            <span>{user.email}</span>
            <button onClick={logout} className={styles.logout}>
              Logout
            </button>
          </div>
        )}
      </div>
    </header>
  )
}
