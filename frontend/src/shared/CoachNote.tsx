import type { Schemas } from '../api/client'
import { Avatar } from './Avatar'

/** A note from the coach, signed with her photo and name like a message. */
export function CoachNote({
  coach,
  text,
}: {
  coach: Schemas['CoachCard'] | null
  text: string
}) {
  return (
    <div className="notice coach-note">
      <span className="coach-note-head">
        {coach && <Avatar name={coach.name} url={coach.avatar_url} small />}
        <strong>{coach?.name ?? 'Тренер'}</strong>
      </span>
      <p>{text}</p>
    </div>
  )
}
