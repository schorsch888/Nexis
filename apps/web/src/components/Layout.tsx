import { ReactNode } from 'react'
import { Link } from 'react-router-dom'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  return (
    <div>
      <nav className="navbar">
        <Link to="/">
          <h1 style={{ margin: 0 }}>Nexis</h1>
        </Link>
        <div>
          <Link to="/">Rooms</Link>
          <a href="https://docs.nexis.ai" target="_blank" rel="noreferrer">Docs</a>
        </div>
      </nav>
      <main>{children}</main>
    </div>
  )
}
