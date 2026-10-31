export default function Logo({ size = 26 }: { size?: number }) {
  return (
    <span className="logo">
      <svg width={size} height={size} viewBox="0 0 32 32" role="img" aria-label="Nodal logo">
        <defs>
          <linearGradient id="nodal-lg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#6366f1" />
            <stop offset="1" stopColor="#3b82f6" />
          </linearGradient>
        </defs>
        <rect width="32" height="32" rx="9" fill="url(#nodal-lg)" />
        <path d="M9 21 L16 11 L23 21" stroke="#fff" strokeWidth="2.2" fill="none" strokeLinecap="round" strokeLinejoin="round" />
        <circle cx="9" cy="21" r="3" fill="#fff" />
        <circle cx="16" cy="11" r="3" fill="#fff" />
        <circle cx="23" cy="21" r="3" fill="#fff" />
      </svg>
      <span>Nodal</span>
    </span>
  )
}
