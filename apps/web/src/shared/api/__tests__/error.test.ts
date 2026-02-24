import { describe, it, expect } from 'vitest'
import { parseApiError } from '../error'

describe('parseApiError', () => {
  it('should handle 401 error', () => {
    const axiosError = {
      isAxiosError: true,
      response: { status: 401, data: {} },
      message: 'Unauthorized',
    } as unknown as Error
    
    const result = parseApiError(axiosError)
    expect(result.code).toBe('UNAUTHORIZED')
    expect(result.status).toBe(401)
  })

  it('should handle generic error', () => {
    const error = new Error('Something went wrong')
    const result = parseApiError(error)
    expect(result.message).toBe('Something went wrong')
  })

  it('should handle unknown error', () => {
    const result = parseApiError(null)
    expect(result.message).toBe('An unexpected error occurred')
  })
})
