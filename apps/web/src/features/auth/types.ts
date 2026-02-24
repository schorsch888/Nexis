export interface LoginCredentials {
  token: string
  memberId: string
  tenantId?: string
}

export interface AuthState {
  token: string | null
  memberId: string | null
  tenantId: string | null
  isAuthenticated: boolean
}
