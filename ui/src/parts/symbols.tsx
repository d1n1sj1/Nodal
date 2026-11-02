import type { FC, ReactNode } from 'react'

const sym = (d: ReactNode): FC => () => (
  <svg width="34" height="18" viewBox="0 0 34 18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {d}
  </svg>
)

export const ResistorSymbol = sym(<path d="M1 9h6l2-6 4 12 4-12 4 12 2-6h10" />)
export const CapacitorSymbol = sym(<path d="M1 9h12M21 9h12M13 2v14M21 2v14" />)
export const InductorSymbol = sym(<path d="M1 12h5a4 4 0 018 0 4 4 0 018 0 4 4 0 018 0h3" />)
export const VSourceSymbol = sym(<path d="M1 9h7M26 9h7M17 1a8 8 0 100 16 8 8 0 000-16zM11 9h4M13 7v4M20 9h4" />)
export const ISourceSymbol = sym(<path d="M1 9h7M26 9h7M17 1a8 8 0 100 16 8 8 0 000-16zM12 9h9M18 6l3 3-3 3" />)
export const DiodeSymbol = sym(<path d="M1 9h12M21 9h12M13 2l8 7-8 7zM21 2v14" />)
export const LedSymbol = sym(<path d="M1 9h10M21 9h12M11 3l8 6-8 6zM19 3v12M17 1l3-1M21 3l3-1" />)
export const SwitchSymbol = sym(<path d="M1 12h9M24 12h9M10 12l12-8" />)
// Pins: base left, collector top right, emitter bottom right (editor/schematic.ts pinPos).
export const NpnSymbol = sym(<path d="M1 9h12M13 3v12M13 6l12-5h8M13 12l12 5h8M20 15.5l5 1.5-1.5-4.5" />)
export const PnpSymbol = sym(<path d="M1 9h12M13 3v12M13 6l12-5h8M13 12l12 5h8M16.5 13.5l-3.5-1.5 1.5 4" />)
// Pins: gate left, drain top right, source bottom right (editor/schematic.ts pinPos).
export const NmosSymbol = sym(<path d="M1 9h8M9 3v12M13 3v3M13 7.5v3M13 12v3M13 4h20v-3M13 14h20v3M13 9h20M28 7l-4 2 4 2" />)
export const PmosSymbol = sym(<path d="M1 9h8M9 3v12M13 3v3M13 7.5v3M13 12v3M13 4h20v-3M13 14h20v3M13 9h20M25 7l4 2-4 2" />)
// Pins: inputs left (+ above -), output right (editor/schematic.ts pinPos).
export const OpampSymbol = sym(<path d="M1 3h8M1 15h8M9 0v18l20-9zM29 9h4M12 5h3M13.5 3.5v3M12 13h3" />)
export const GroundSymbol = sym(<path d="M17 1v6M9 7h16M12 11h10M15 15h4" />)
