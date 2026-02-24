import { NavLink } from 'react-router-dom'
import styles from './SideNav.module.css'

export function SideNav() {
  return (
    <nav className={styles.sidenav}>
      <div className={styles.section}>
        <h3 className={styles.title}>Navigation</h3>
        <NavLink to="/app/rooms" className={({ isActive }) => isActive ? styles.activeLink : styles.link}>
          Rooms
        </NavLink>
        <NavLink to="/app/members" className={({ isActive }) => isActive ? styles.activeLink : styles.link}>
          Members
        </NavLink>
        <NavLink to="/app/search" className={({ isActive }) => isActive ? styles.activeLink : styles.link}>
          Search
        </NavLink>
      </div>
    </nav>
  )
}
