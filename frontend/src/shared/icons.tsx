import type { ReactNode } from 'react'

/** Stroke icons in the prototype's style; they take the text color. */
function Icon({ size = 18, children }: { size?: number; children: ReactNode }) {
  return (
    <svg
      className="icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </svg>
  )
}

export function PlusIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M12 5v14M5 12h14" strokeWidth={2.4} />
    </Icon>
  )
}

export function PeopleIcon({ size }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM2 21a7 7 0 0 1 14 0M16 3.5a4 4 0 0 1 0 7.5M22 21a7 7 0 0 0-5-6.7" />
    </Icon>
  )
}

export function ImportIcon({ size }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M6 3h9l4 4v14H6zM9 12h6M9 16h6" />
    </Icon>
  )
}

export function SendIcon({ size }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M21 3L3 10.5l7 2.5 2.5 7L21 3zM10 13l5-5" strokeWidth={2} />
    </Icon>
  )
}

export function SearchIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" strokeWidth={2} />
    </Icon>
  )
}

export function BackIcon({ size = 20 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M15 5l-7 7 7 7" strokeWidth={2.2} />
    </Icon>
  )
}
