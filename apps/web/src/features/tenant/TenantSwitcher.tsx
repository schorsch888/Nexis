import { useTenantStore } from './tenantStore'
import styles from './TenantSwitcher.module.css'

export function TenantSwitcher() {
  const { tenantId, setTenantId, availableTenants } = useTenantStore()

  if (!tenantId && availableTenants.length === 0) {
    return null
  }

  return (
    <div className={styles.switcher}>
      <select
        value={tenantId || ''}
        onChange={(e) => setTenantId(e.target.value)}
        className={styles.select}
      >
        {availableTenants.map((t) => (
          <option key={t.id} value={t.id}>
            {t.name}
          </option>
        ))}
      </select>
    </div>
  )
}
