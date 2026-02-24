import { describe, it, expect, beforeEach } from 'vitest'
import { useAuthStore } from '../authStore'

describe('authStore', () => {
  beforeEach(() => {
    useAuthStore.setState({
      token: null,
      memberId: null,
      tenantId: null,
      user: null,
      isAuthenticated: false,
    })
  })

  it('should start with default values', () => {
    const state = useAuthStore.getState()
    expect(state.token).toBeNull()
    expect(state.memberId).toBeNull()
    expect(state.isAuthenticated).toBe(false)
  })

  it('should login successfully', () => {
    const { login } = useAuthStore.getState()
    login({
      token: 'test-token',
      memberId: 'member-123',
      tenantId: 'tenant-456',
    })
    
    const state = useAuthStore.getState()
    expect(state.token).toBe('test-token')
    expect(state.memberId).toBe('member-123')
    expect(state.tenantId).toBe('tenant-456')
    expect(state.isAuthenticated).toBe(true)
  })

  it('should logout successfully', () => {
    const { login, logout } = useAuthStore.getState()
    login({
      token: 'test-token',
      memberId: 'member-123',
    })
    
    logout()
    
    const state = useAuthStore.getState()
    expect(state.token).toBeNull()
    expect(state.memberId).toBeNull()
    expect(state.isAuthenticated).toBe(false)
  })

  it('should update tenant id', () => {
    const { login, setTenantId } = useAuthStore.getState()
    login({
      token: 'test-token',
      memberId: 'member-123',
    })
    
    setTenantId('new-tenant')
    
    const state = useAuthStore.getState()
    expect(state.tenantId).toBe('new-tenant')
  })
})
