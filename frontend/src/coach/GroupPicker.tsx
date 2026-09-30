import { MUSCLE_GROUPS } from './library'

export function GroupChip({
  label,
  selected,
  onClick,
}: {
  label: string
  selected: boolean
  onClick: () => void
}) {
  return (
    <button type="button" className="chip" aria-pressed={selected} onClick={onClick}>
      {label}
    </button>
  )
}

/** Picks one muscle group; tapping the chosen one again clears it. */
export function GroupPicker({
  value,
  onChange,
}: {
  value: string | null
  onChange: (group: string | null) => void
}) {
  // Keep a group set elsewhere (e.g. by an import) visible and selectable.
  const groups = value && !MUSCLE_GROUPS.includes(value) ? [...MUSCLE_GROUPS, value] : MUSCLE_GROUPS
  return (
    <div className="chips wrap" role="group" aria-label="Група м’язів">
      {groups.map((group) => (
        <GroupChip
          key={group}
          label={group}
          selected={value === group}
          onClick={() => onChange(value === group ? null : group)}
        />
      ))}
    </div>
  )
}
