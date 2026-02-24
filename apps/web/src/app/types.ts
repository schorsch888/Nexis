export interface User {
  id: string
  email: string
  name?: string
}

export interface Session {
  token: string
  memberId: string
  tenantId?: string
  user?: User
}
