import { useState, useEffect } from 'react'
import { useMembersStore } from './membersStore'
import { MemberTable } from './MemberTable'
import { InviteMemberDialog } from './InviteMemberDialog'
import styles from './MembersPage.module.css'

export function MembersPage() {
  const { members, loading, error, fetchMembers } = useMembersStore()
  const [showInvite, setShowInvite] = useState(false)

  useEffect(() => {
    fetchMembers()
  }, [fetchMembers])

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h2>Members</h2>
        <button onClick={() => setShowInvite(true)} className={styles.inviteBtn}>
          Invite Member
        </button>
      </div>

      {error && <div className={styles.error}>{error}</div>}

      {loading ? (
        <div className={styles.loading}>Loading members...</div>
      ) : (
        <MemberTable members={members} />
      )}

      {showInvite && <InviteMemberDialog onClose={() => setShowInvite(false)} />}
    </div>
  )
}

export default MembersPage
