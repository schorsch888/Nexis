import { useState } from 'react'
import { useMembersStore } from './membersStore'
import styles from './InviteMemberDialog.module.css'

interface InviteMemberDialogProps {
  onClose: () => void
}

export function InviteMemberDialog({ onClose }: InviteMemberDialogProps) {
  const { inviteMember } = useMembersStore()
  const [email, setEmail] = useState('')
  const [role, setRole] = useState('member')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!email.trim()) {
      setError('Email is required')
      return
    }
    setLoading(true)
    setError(null)
    const success = await inviteMember(email.trim(), role)
    setLoading(false)
    if (success) {
      onClose()
    } else {
      setError('Failed to send invite')
    }
  }

  return (
    <div className={styles.overlay}>
      <div className={styles.dialog}>
        <h3>Invite Member</h3>
        <form onSubmit={handleSubmit}>
          {error && <div className={styles.error}>{error}</div>}
          <div className={styles.field}>
            <label htmlFor="email">Email</label>
            <input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="member@example.com"
            />
          </div>
          <div className={styles.field}>
            <label htmlFor="role">Role</label>
            <select id="role" value={role} onChange={(e) => setRole(e.target.value)}>
              <option value="member">Member</option>
              <option value="admin">Admin</option>
            </select>
          </div>
          <div className={styles.actions}>
            <button type="button" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" disabled={loading}>
              {loading ? 'Sending...' : 'Send Invite'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
