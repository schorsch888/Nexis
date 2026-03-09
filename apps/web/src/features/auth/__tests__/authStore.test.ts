import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useAuthStore, resetAuthStore, setRememberMe } from '../authStore'
import type { Session } from '../../../app/types'

const createMockSession = (overrides?: Partial<Session> & {
  expiresAt?: number
  refreshExpiresAt?: number
  roles?: string[]
  permissions?: string[]
}): Session & {
  expiresAt?: number
  refreshExpiresAt?: number
  roles?: string[]
  permissions?: string[]
} => ({
  token: 'test-token',
  memberId: 'member-123',
  tenantId: 'tenant-456',
  ...overrides,
})

describe('authStore', () => {
  beforeEach(() => {
    resetAuthStore()
    localStorage.clear()
    sessionStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('basic auth flow', () => {
    it('should start with default values', () => {
      const state = useAuthStore.getState()
      expect(state.token).toBeNull()
      expect(state.memberId).toBeNull()
      expect(state.isAuthenticated).toBe(false)
    })

    it('should login successfully', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession())

      const state = useAuthStore.getState()
      expect(state.token).toBe('test-token')
      expect(state.memberId).toBe('member-123')
      expect(state.tenantId).toBe('tenant-456')
      expect(state.isAuthenticated).toBe(true)
    })

    it('should logout successfully', () => {
      const { login, logout } = useAuthStore.getState()
      login(createMockSession())

      logout()

      const state = useAuthStore.getState()
      expect(state.token).toBeNull()
      expect(state.memberId).toBeNull()
      expect(state.isAuthenticated).toBe(false)
    })

    it('should update tenant id', () => {
      const { login, setTenantId } = useAuthStore.getState()
      login(createMockSession())

      setTenantId('new-tenant')

      const state = useAuthStore.getState()
      expect(state.tenantId).toBe('new-tenant')
    })
  })

  describe('session model extensions', () => {
    it('should store expiresAt from login', () => {
      const expiresAt = Date.now() + 3600000
      const { login } = useAuthStore.getState()
      login(createMockSession({ expiresAt }))

      const state = useAuthStore.getState()
      expect(state.expiresAt).toBe(expiresAt)
    })

    it('should store roles and permissions from login', () => {
      const roles = ['admin', 'user']
      const permissions = ['read:all', 'write:all']
      const { login } = useAuthStore.getState()
      login(createMockSession({ roles, permissions }))

      const state = useAuthStore.getState()
      expect(state.roles).toEqual(roles)
      expect(state.permissions).toEqual(permissions)
    })

    it('should have session status', () => {
      const state = useAuthStore.getState()
      expect(state.status).toBe('anonymous')

      const { login } = useAuthStore.getState()
      login(createMockSession())

      expect(useAuthStore.getState().status).toBe('authenticated')
    })

    it('should track lastActivityAt', () => {
      const beforeLogin = Date.now()
      const { login } = useAuthStore.getState()
      login(createMockSession())

      const state = useAuthStore.getState()
      expect(state.lastActivityAt).toBeGreaterThanOrEqual(beforeLogin)
    })
  })

  describe('rememberMe storage strategy', () => {
    it('should use sessionStorage when rememberMe is false', () => {
      setRememberMe(false)
      const { login } = useAuthStore.getState()
      login(createMockSession())

      expect(sessionStorage.getItem('nexis-auth')).not.toBeNull()
      expect(localStorage.getItem('nexis-auth')).toBeNull()
    })

    it('should use localStorage when rememberMe is true', () => {
      setRememberMe(true)
      const { login } = useAuthStore.getState()
      login(createMockSession())

      expect(localStorage.getItem('nexis-auth')).not.toBeNull()
      expect(sessionStorage.getItem('nexis-auth')).toBeNull()
    })

    it('should default to sessionStorage (rememberMe=false)', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession())

      expect(sessionStorage.getItem('nexis-auth')).not.toBeNull()
    })

    it('should switch storage on rememberMe change', () => {
      setRememberMe(false)
      const { login } = useAuthStore.getState()
      login(createMockSession({ token: 'session-token' }))

      expect(sessionStorage.getItem('nexis-auth')).toContain('session-token')

      useAuthStore.getState().logout()
      setRememberMe(true)
      login(createMockSession({ token: 'local-token' }))

      expect(localStorage.getItem('nexis-auth')).toContain('local-token')
    })
  })

  describe('permission checking', () => {
    it('hasPermission should return true when user has required permission', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ permissions: ['read:all', 'write:own'] }))

      const { hasPermission } = useAuthStore.getState()
      expect(hasPermission('read:all')).toBe(true)
      expect(hasPermission('write:own')).toBe(true)
    })

    it('hasPermission should return false when user lacks permission', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ permissions: ['read:own'] }))

      const { hasPermission } = useAuthStore.getState()
      expect(hasPermission('write:all')).toBe(false)
    })

    it('hasRole should return true when user has required role', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ roles: ['admin', 'moderator'] }))

      const { hasRole } = useAuthStore.getState()
      expect(hasRole('admin')).toBe(true)
    })

    it('hasRole should return false when user lacks role', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ roles: ['user'] }))

      const { hasRole } = useAuthStore.getState()
      expect(hasRole('admin')).toBe(false)
    })
  })

  describe('token expiry detection', () => {
    it('isTokenExpired should return true when token is expired', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ expiresAt: Date.now() - 1000 }))

      const { isTokenExpired } = useAuthStore.getState()
      expect(isTokenExpired()).toBe(true)
    })

    it('isTokenExpired should return false when token is valid', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ expiresAt: Date.now() + 3600000 }))

      const { isTokenExpired } = useAuthStore.getState()
      expect(isTokenExpired()).toBe(false)
    })

    it('needsRefresh should return true when token expires soon', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ expiresAt: Date.now() + 30000 }))

      const { needsRefresh } = useAuthStore.getState()
      expect(needsRefresh()).toBe(true)
    })

    it('needsRefresh should return false when token has plenty of time', () => {
      const { login } = useAuthStore.getState()
      login(createMockSession({ expiresAt: Date.now() + 3600000 }))

      const { needsRefresh } = useAuthStore.getState()
      expect(needsRefresh()).toBe(false)
    })
  })
})
