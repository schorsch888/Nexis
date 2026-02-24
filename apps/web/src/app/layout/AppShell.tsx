import { Outlet } from 'react-router-dom'
import { TopBar } from './TopBar'
import { SideNav } from './SideNav'
import styles from './AppShell.module.css'

export function AppShell() {
  return (
    <div className={styles.shell}>
      <TopBar />
      <div className={styles.body}>
        <SideNav />
        <main className={styles.content}>
          <Outlet />
        </main>
      </div>
    </div>
  )
}
