import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Session, User } from '../../app/types'

interface AuthState {
  token: string | null
  memberId: string | null
  tenantId: string | null
  user: User | null
  isAuthenticated: boolean
  login: (session: Session) => void
  logout: () => void
  setTenantId: (tenantId: string) => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      memberId: null,
      tenantId: null,
      user: null,
      isAuthenticated: false,
      login: (session: Session) =>
        set({
          token: session.token,
          memberId: session.memberId,
          tenantId: session.tenantId || null,
          user: session.user || null,
          isAuthenticated: true,
        }),
      logout: () =>
        set({
          token: null,
          memberId: null,
          tenantId: null,
          user: null,
          isAuthenticated: false,
        }),
      setTenantId: (tenantId: string) => set({ tenantId }),
    }),
    {
      name: 'nexis-auth',
    }
  )
)
