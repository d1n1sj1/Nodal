import type { FC, ReactNode } from 'react'

const sym = (d: ReactNode): FC => () => (
  <svg width="34" height="18" viewBox="0 0 34 18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {d}
  </svg>
)

export const ResistorSymbol = sym(<path d="M1 9h6l2-6 4 12 4-12 4 12 2-6h10" />)
export const CapacitorSymbol = sym(<path d="M1 9h12M21 9h12M13 2v14M21 2v14" />)
export const InductorSymbol = sym(<path d="M1 12h5a4 4 0 018 0 4 4 0 018 0 4 4 0 018 0h3" />)
export const VSourceSymbol = sym(<path d="M1 9h7M26 9h7M17 1a8 8 0 100 16 8 8 0 000-16zM13 9h8M17 5v8" />)
export const ISourceSymbol = sym(<path d="M1 9h7M26 9h7M17 1a8 8 0 100 16 8 8 0 000-16zM12 9h9M18 6l3 3-3 3" />)
export const DiodeSymbol = sym(<path d="M1 9h12M21 9h12M13 2l8 7-8 7zM21 2v14" />)
export const LedSymbol = sym(<path d="M1 9h10M21 9h12M11 3l8 6-8 6zM19 3v12M17 1l3-1M21 3l3-1" />)
export const SwitchSymbol = sym(<path d="M1 12h9M24 12h9M10 12l12-8" />)
export const GroundSymbol = sym(<path d="M17 1v6M9 7h16M12 11h10M15 15h4" />)
