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
      {/* Length 1, so a check can draw itself (see `.set-check.ticked`). */}
      <path d="M5 12.5l4.5 4.5L19 7" strokeWidth={2.6} pathLength={1} />
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

export function PersonIcon({ size = 20 }: { size?: number }) {
  return (
    <Icon size={size}>
      <circle cx="12" cy="8" r="4" />
      <path d="M4.5 20c1.4-3.4 4.2-5 7.5-5s6.1 1.6 7.5 5" />
    </Icon>
  )
}

export function PhotoIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M4 8h3.2l1.8-2.5h6l1.8 2.5H20v11H4z" />
      <circle cx="12" cy="13" r="3.5" />
    </Icon>
  )
}

export function ScaleIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <rect x="4" y="4" width="16" height="16" rx="4" />
      <path d="M8.5 10a3.5 3.5 0 0 1 7 0" />
      <path d="M12 10l1.4-1.8" />
    </Icon>
  )
}

export function PlateIcon({ size = 16 }: { size?: number }) {
  return (
    <Icon size={size}>
      <circle cx="12" cy="12" r="8" />
      <circle cx="12" cy="12" r="4" />
    </Icon>
  )
}

/** iPhone's "Поділитися": an arrow out of a box. */
export function ShareIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M12 3v12M8 7l4-4 4 4M7 11H5v10h14V11h-2" />
    </Icon>
  )
}

/** iPhone's "На початковий екран": a plus in a box. */
export function AddBoxIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <rect x="4" y="4" width="16" height="16" rx="4" />
      <path d="M12 8v8M8 12h8" />
    </Icon>
  )
}

/** Chrome's menu on Android: three dots, one above another. */
export function MenuDotsIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <path d="M12 5h.01M12 12h.01M12 19h.01" strokeWidth={3} />
    </Icon>
  )
}

/** A phone, for the app on its home screen. */
export function PhoneIcon({ size = 18 }: { size?: number }) {
  return (
    <Icon size={size}>
      <rect x="6.5" y="3" width="11" height="18" rx="2.5" />
      <path d="M11 17.5h2" />
    </Icon>
  )
}
