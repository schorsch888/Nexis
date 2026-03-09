import { create, StateCreator } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { Session, User, SessionStatus } from '../../app/types'

const REFRESH_THRESHOLD_MS = 60 * 1000
const STORAGE_KEY = 'nexis-auth'

interface AuthState {
  token: string | null
  memberId: string | null
  tenantId: string | null
  user: User | null
  isAuthenticated: boolean
  expiresAt: number | null
  refreshExpiresAt: number | null
  lastActivityAt: number | null
  status: SessionStatus
  roles: string[]
  permissions: string[]
  rememberMe: boolean
  login: (session: Session) => void
  logout: () => void
  setTenantId: (tenantId: string) => void
  updateSession: (session: Partial<Session>) => void
  updateActivity: () => void
  setStatus: (status: SessionStatus) => void
  hasPermission: (permission: string) => boolean
  hasRole: (role: string) => boolean
  isTokenExpired: () => boolean
  needsRefresh: () => boolean
  setRememberMe: (value: boolean) => void
}

type AuthPersistState = {
  token: string | null
  memberId: string | null
  tenantId: string | null
  user: User | null
  isAuthenticated: boolean
  expiresAt: number | null
  refreshExpiresAt: number | null
  lastActivityAt: number | null
  status: SessionStatus
  roles: string[]
  permissions: string[]
  rememberMe: boolean
}

type AuthStoreCreator = StateCreator<AuthState, [], [['zustand/persist', AuthPersistState]]>

function getTargetStorage(rememberMe: boolean): Storage {
  return rememberMe ? localStorage : sessionStorage
}

function createAuthStore() {
  const authStoreCreator: AuthStoreCreator = (set, get) => ({
    token: null,
    memberId: null,
    tenantId: null,
    user: null,
    isAuthenticated: false,
    expiresAt: null,
    refreshExpiresAt: null,
    lastActivityAt: null,
    status: 'anonymous',
    roles: [],
    permissions: [],
    rememberMe: false,

    login: (session: Session) => {
      set({
        token: session.token,
        memberId: session.memberId,
        tenantId: session.tenantId || null,
        user: session.user || null,
        isAuthenticated: true,
        expiresAt: session.expiresAt ?? null,
        refreshExpiresAt: session.refreshExpiresAt ?? null,
        lastActivityAt: Date.now(),
        status: 'authenticated',
        roles: session.roles ?? [],
        permissions: session.permissions ?? [],
      })
    },

    logout: () => {
      set({
        token: null,
        memberId: null,
        tenantId: null,
        user: null,
        isAuthenticated: false,
        expiresAt: null,
        refreshExpiresAt: null,
        lastActivityAt: null,
        status: 'anonymous',
        roles: [],
        permissions: [],
      })
      localStorage.removeItem(STORAGE_KEY)
      sessionStorage.removeItem(STORAGE_KEY)
    },

    setTenantId: (tenantId: string) => set({ tenantId }),

    updateSession: (session: Partial<Session>) => {
      set((state) => ({
        token: session.token ?? state.token,
        memberId: session.memberId ?? state.memberId,
        tenantId: session.tenantId ?? state.tenantId,
        user: session.user ?? state.user,
        expiresAt: session.expiresAt ?? state.expiresAt,
        refreshExpiresAt: session.refreshExpiresAt ?? state.refreshExpiresAt,
        roles: session.roles ?? state.roles,
        permissions: session.permissions ?? state.permissions,
        lastActivityAt: Date.now(),
      }))
    },

    updateActivity: () => set({ lastActivityAt: Date.now() }),

    setStatus: (status: SessionStatus) => set({ status }),

    hasPermission: (permission: string) => {
      const { permissions } = get()
      return permissions.includes(permission)
    },

    hasRole: (role: string) => {
      const { roles } = get()
      return roles.includes(role)
    },

    isTokenExpired: () => {
      const { expiresAt } = get()
      if (!expiresAt) return false
      return Date.now() >= expiresAt
    },

    needsRefresh: () => {
      const { expiresAt } = get()
      if (!expiresAt) return false
      return Date.now() >= expiresAt - REFRESH_THRESHOLD_MS
    },

    setRememberMe: (value: boolean) => {
      const currentRememberMe = get().rememberMe
      if (currentRememberMe === value) return

      const oldData = getTargetStorage(currentRememberMe).getItem(STORAGE_KEY)
      getTargetStorage(currentRememberMe).removeItem(STORAGE_KEY)

      set({ rememberMe: value })

      if (oldData) {
        getTargetStorage(value).setItem(STORAGE_KEY, oldData)
      }
    },
  })

  const dynamicStorage = createJSONStorage(() => ({
    getItem: (name: string): string | null => {
      const localData = localStorage.getItem(name)
      if (localData) {
        try {
          const parsed = JSON.parse(localData)
          if (parsed?.state?.rememberMe) {
            sessionStorage.removeItem(name)
            return localData
          }
        } catch {}
      }

      const sessionData = sessionStorage.getItem(name)
      if (sessionData) {
        localStorage.removeItem(name)
        return sessionData
      }

      return null
    },
    setItem: (name: string, value: string): void => {
      let rememberMe = false
      try {
        const parsed = JSON.parse(value)
        rememberMe = parsed?.state?.rememberMe ?? false
      } catch {}

      const targetStorage = getTargetStorage(rememberMe)
      const otherStorage = getTargetStorage(!rememberMe)

      targetStorage.setItem(name, value)
      otherStorage.removeItem(name)
    },
    removeItem: (name: string): void => {
      localStorage.removeItem(name)
      sessionStorage.removeItem(name)
    },
  }))

  return create<AuthState>()(
    persist(authStoreCreator, {
      name: STORAGE_KEY,
      storage: dynamicStorage,
      partialize: (state): AuthPersistState => ({
        token: state.token,
        memberId: state.memberId,
        tenantId: state.tenantId,
        user: state.user,
        isAuthenticated: state.isAuthenticated,
        expiresAt: state.expiresAt,
        refreshExpiresAt: state.refreshExpiresAt,
        lastActivityAt: state.lastActivityAt,
        status: state.status,
        roles: state.roles,
        permissions: state.permissions,
        rememberMe: state.rememberMe,
      }),
    })
  )
}

let storeInstance: ReturnType<typeof createAuthStore> | null = null

function getStore() {
  if (!storeInstance) {
    storeInstance = createAuthStore()
  }
  return storeInstance
}

export const useAuthStore = new Proxy({} as ReturnType<typeof createAuthStore>, {
  get(_, prop) {
    const store = getStore()
    if (!store) return undefined
    return store[prop as keyof typeof store]
  },
})

export function resetAuthStore(): void {
  localStorage.removeItem(STORAGE_KEY)
  sessionStorage.removeItem(STORAGE_KEY)
  storeInstance = null
}

export function setRememberMe(value: boolean): void {
  const store = getStore()
  if (store) {
    store.getState().setRememberMe(value)
  }
}
