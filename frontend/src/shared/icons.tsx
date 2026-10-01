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

export function LibraryIcon({ size }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M4 5h16v14H4zM10 9v6l5-3z" />
    </Icon>
  )
}

export function CameraIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M3 7h12v10H3zM15 10l6-3v10l-6-3" />
    </Icon>
  )
}

export function UploadIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M12 16V4M7 9l5-5 5 5M4 20h16" strokeWidth={2} />
    </Icon>
  )
}

export function MinusIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M5 12h14" strokeWidth={2.4} />
    </Icon>
  )
}

export function MoreIcon({ size = 20 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M5 12h.01M12 12h.01M19 12h.01" strokeWidth={3} />
    </Icon>
  )
}

export function CloseIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M6 6l12 12M18 6L6 18" strokeWidth={2.2} />
    </Icon>
  )
}

export function CheckIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M5 12.5l4.5 4.5L19 7" strokeWidth={2.6} />
    </Icon>
  )
}

export function CopyIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M8 8h12v12H8zM4 16V4h12" />
    </Icon>
  )
}

export function CalendarIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M4 6h16v14H4zM4 10h16M8 3v4M16 3v4" />
    </Icon>
  )
}

export function ChevronIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M9 5l7 7-7 7" strokeWidth={2.2} />
    </Icon>
  )
}

export function EditIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" />
    </Icon>
  )
}
