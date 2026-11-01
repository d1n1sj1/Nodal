import type { FC, ReactNode } from 'react'
import type { PartDef } from './registry'

export interface KidsTile {
  name: string // friendly name shown under the picture
  color: string // tile background (never yellow)
  picture: FC
}

const pic = (d: ReactNode): FC => () => (
  <svg width="44" height="44" viewBox="0 0 44 44" fill="none" stroke="#1e293b" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {d}
  </svg>
)

export const tiles = {
  battery: { name: 'Battery', color: '#d1fae5', picture: pic(<><rect x="6" y="14" width="28" height="16" rx="3" fill="#34d399" /><path d="M34 19h4v6h-4M14 22h6M17 19v6" /></>) },
  bulb: { name: 'Bulb', color: '#fecdd3', picture: pic(<><path d="M22 6a11 11 0 00-6 20v4h12v-4a11 11 0 00-6-20z" fill="#fb7185" /><path d="M17 36h10M19 40h6" /></>) },
  switch: { name: 'Switch', color: '#dbeafe', picture: pic(<><circle cx="9" cy="30" r="3" fill="#3b82f6" /><circle cx="35" cy="30" r="3" fill="#3b82f6" /><path d="M9 30l22-14" /></>) },
  buzzer: { name: 'Buzzer', color: '#fbcfe8', picture: pic(<><path d="M8 18h8l9-7v22l-9-7H8z" fill="#f472b6" /><path d="M30 16q4 6 0 12M34 12q8 10 0 20" /></>) },
  resistor: { name: 'Resistor', color: '#e0e7ff', picture: pic(<path d="M3 22h8l3-9 6 18 6-18 6 18 3-9h6" stroke="#6366f1" />) },
  motor: { name: 'Motor', color: '#ede9fe', picture: pic(<><circle cx="22" cy="22" r="14" fill="#a78bfa" /><path d="M22 22V10M22 22l10 6M22 22l-10 6" /></>) },
} satisfies Record<string, KidsTile>

const GenericPicture = pic(<><rect x="8" y="8" width="28" height="28" rx="8" /><path d="M16 22h12" /></>)

export function tileFor(def: PartDef): KidsTile {
  const byKind: Partial<Record<PartDef['kind'], KidsTile>> = { VSource: tiles.battery, Switch: tiles.switch, Resistor: tiles.resistor }
  return def.kidsTile ?? byKind[def.kind] ?? { name: def.name, color: '#e2e8f0', picture: GenericPicture }
}
