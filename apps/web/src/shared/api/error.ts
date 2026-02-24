export interface ApiError {
  message: string
  code?: string
  status?: number
}

export function parseApiError(error: unknown): ApiError {
  if (axios.isAxiosError(error)) {
    const status = error.response?.status
    const data = error.response?.data as { message?: string; code?: string } | undefined
    
    if (status === 401) {
      return { message: 'Authentication required', code: 'UNAUTHORIZED', status }
    }
    if (status === 403) {
      return { message: 'Access denied', code: 'FORBIDDEN', status }
    }
    if (status === 404) {
      return { message: 'Resource not found', code: 'NOT_FOUND', status }
    }
    if (status && status >= 500) {
      return { message: 'Server error. Please try again.', code: 'SERVER_ERROR', status }
    }
    
    return {
      message: data?.message || error.message || 'An error occurred',
      code: data?.code,
      status,
    }
  }
  
  if (error instanceof Error) {
    return { message: error.message }
  }
  
  return { message: 'An unexpected error occurred' }
}

import axios from 'axios'
