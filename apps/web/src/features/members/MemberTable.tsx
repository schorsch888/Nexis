import type { Member } from './membersStore'
import styles from './MemberTable.module.css'

interface MemberTableProps {
  members: Member[]
}

export function MemberTable({ members }: MemberTableProps) {
  if (members.length === 0) {
    return <div className={styles.empty}>No members yet</div>
  }

  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th>Name</th>
          <th>Email</th>
          <th>Role</th>
          <th>Joined</th>
        </tr>
      </thead>
      <tbody>
        {members.map((member) => (
          <tr key={member.id || member.email}>
            <td>{member.name || '—'}</td>
            <td>{member.email}</td>
            <td>{member.role}</td>
            <td>{member.joinedAt ? new Date(member.joinedAt).toLocaleDateString() : '—'}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}
