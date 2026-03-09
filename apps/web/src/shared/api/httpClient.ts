import axios, { AxiosError, AxiosInstance, InternalAxiosRequestConfig } from 'axios'
import { useAuthStore } from '../../features/auth/authStore'
import { sessionManager, isRecoverable401, handleRefreshSuccess, handleRefreshFailure } from './sessionManager'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api/v1'

class HttpClient {
  private instance: AxiosInstance

  constructor() {
    this.instance = axios.create({
      baseURL: API_BASE_URL,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    })

    this.setupInterceptors()
  }

  // Reserved for future token refresh implementation
  // eslint-disable-next-line
  private async _refreshAccessToken(): Promise<string | null> {
    try {
      const response = await axios.post<{ token: string; expiresAt: number }>(
        `${API_BASE_URL}/auth/refresh`,
        {},
        { withCredentials: true }
      )

      const { token, expiresAt } = response.data
      handleRefreshSuccess({ token, expiresAt })
      return token
    } catch (error) {
      handleRefreshFailure()
      return null
    }
  }

  private setupInterceptors() {
    this.instance.interceptors.request.use(
      async (config: InternalAxiosRequestConfig) => {
        const state = useAuthStore.getState()
        const { token, tenantId, needsRefresh, isAuthenticated } = state

        if (isAuthenticated && needsRefresh() && !config.url?.includes('/auth/')) {
          const newToken = await sessionManager.executeWithRefreshLock(async () => {
            const response = await axios.post<{ token: string; expiresAt: number }>(
              `${API_BASE_URL}/auth/refresh`,
              {},
              { withCredentials: true }
            )
            return response.data
          })

          if (newToken) {
            handleRefreshSuccess(newToken)
            config.headers.Authorization = `Bearer ${newToken.token}`
          }
        } else if (token) {
          config.headers.Authorization = `Bearer ${token}`
        }

        if (tenantId) {
          config.headers['X-Tenant-ID'] = tenantId
        }

        return config
      },
      (error) => Promise.reject(error)
    )

    this.instance.interceptors.response.use(
      (response) => response,
      async (error: AxiosError<{ code?: string }>) => {
        const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean }

        if (error.response?.status === 401) {
          const errorData = error.response.data

          if (isRecoverable401(errorData) && originalRequest && !originalRequest._retry) {
            originalRequest._retry = true

            try {
              const result = await sessionManager.executeWithRefreshLock(async () => {
                const response = await axios.post<{ token: string; expiresAt: number }>(
                  `${API_BASE_URL}/auth/refresh`,
                  {},
                  { withCredentials: true }
                )
                return response.data
              })

              handleRefreshSuccess(result)

              originalRequest.headers.Authorization = `Bearer ${result.token}`
              return this.instance(originalRequest)
            } catch (refreshError) {
              handleRefreshFailure()
              return Promise.reject(refreshError)
            }
          }

          useAuthStore.getState().logout()
          if (typeof window !== 'undefined') {
            window.location.href = '/login'
          }
        }

        return Promise.reject(error)
      }
    )
  }

  get<T>(url: string, params?: Record<string, unknown>) {
    return this.instance.get<T>(url, { params })
  }

  post<T>(url: string, data?: unknown) {
    return this.instance.post<T>(url, data)
  }

  put<T>(url: string, data?: unknown) {
    return this.instance.put<T>(url, data)
  }

  patch<T>(url: string, data?: unknown) {
    return this.instance.patch<T>(url, data)
  }

  delete<T>(url: string) {
    return this.instance.delete<T>(url)
  }
}

export const httpClient = new HttpClient()
