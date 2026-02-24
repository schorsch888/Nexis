import { httpClient } from '../httpClient'

export interface Member {
  id: string
  email: string
  name?: string
  role: string
  joinedAt?: string
}

export const membersApi = {
  list: () => httpClient.get<Member[]>('/members'),
  invite: (email: string, role?: string) =>
    httpClient.post('/members/invite', { email, role }),
}
