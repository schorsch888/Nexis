import { useNavigate, useLocation } from 'react-router-dom'
import { useState } from 'react'
import { useAuthStore } from './authStore'
import type { LoginCredentials } from './types'
import styles from './LoginPage.module.css'

export function LoginPage() {
  const navigate = useNavigate()
  const location = useLocation()
  const { login } = useAuthStore()
  const [credentials, setCredentials] = useState<LoginCredentials>({
    token: '',
    memberId: '',
    tenantId: '',
  })
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const from = (location.state as { from?: { pathname: string } })?.from?.pathname || '/app/rooms'

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)

    if (!credentials.token.trim()) {
      setError('Token is required')
      return
    }
    if (!credentials.memberId.trim()) {
      setError('Member ID is required')
      return
    }

    setLoading(true)
    try {
      login({
        token: credentials.token.trim(),
        memberId: credentials.memberId.trim(),
        tenantId: credentials.tenantId?.trim() || undefined,
      })
      navigate(from, { replace: true })
    } catch (err) {
      setError('Login failed. Please check your credentials.')
    } finally {
      setLoading(false)
    }
  }

  const handleChange = (field: keyof LoginCredentials) => (
    e: React.ChangeEvent<HTMLInputElement>
  ) => {
    setCredentials((prev) => ({ ...prev, [field]: e.target.value }))
  }

  return (
    <div className={styles.container}>
      <div className={styles.card}>
        <h1 className={styles.title}>Sign in to Nexis</h1>
        <p className={styles.subtitle}>Enter your credentials to continue</p>

        {error && <div className={styles.error}>{error}</div>}

        <form onSubmit={handleSubmit} className={styles.form}>
          <div className={styles.field}>
            <label htmlFor="token">Token</label>
            <input
              id="token"
              type="password"
              value={credentials.token}
              onChange={handleChange('token')}
              placeholder="Enter your API token"
              autoComplete="off"
            />
          </div>

          <div className={styles.field}>
            <label htmlFor="memberId">Member ID</label>
            <input
              id="memberId"
              type="text"
              value={credentials.memberId}
              onChange={handleChange('memberId')}
              placeholder="Enter your member ID"
            />
          </div>

          <div className={styles.field}>
            <label htmlFor="tenantId">
              Tenant ID <span className={styles.optional}>(optional)</span>
            </label>
            <input
              id="tenantId"
              type="text"
              value={credentials.tenantId || ''}
              onChange={handleChange('tenantId')}
              placeholder="Enter tenant ID"
            />
          </div>

          <button type="submit" disabled={loading} className={styles.submit}>
            {loading ? 'Signing in...' : 'Sign in'}
          </button>
        </form>
      </div>
    </div>
  )
}

export default LoginPage
