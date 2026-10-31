import type { FC } from 'react'
import type { PartJson } from '../sim/api'
import * as S from './symbols'
import { tiles, type KidsTile } from './tiles'

export type PartKindName = PartJson['kind']

/** One core component a palette item expands to (Task 11 buildCircuit places `builds` in series). */
export interface PartBuild { kind: PartKindName; defaults: Record<string, number> }

export interface PartDef {
  id: string // unique palette key
  kind: PartKindName | 'Ground' // Ground is a schematic element (node 0), not a core kind
  name: string
  symbol: FC
  kidsTile?: KidsTile
  defaults: Record<string, number>
  builds?: PartBuild[] // kids parts only; absent means a single `kind` part with `defaults`
}

export const partDefs: PartDef[] = [
  { id: 'resistor', kind: 'Resistor', name: 'Resistor', symbol: S.ResistorSymbol, defaults: { ohms: 1000, max_watts: 0.25 } },
  { id: 'capacitor', kind: 'Capacitor', name: 'Capacitor', symbol: S.CapacitorSymbol, defaults: { farads: 1e-6 } },
  { id: 'inductor', kind: 'Inductor', name: 'Inductor', symbol: S.InductorSymbol, defaults: { henries: 1e-3 } },
  { id: 'vsource', kind: 'VSource', name: 'Voltage source', symbol: S.VSourceSymbol, defaults: { volts: 5 } },
  { id: 'isource', kind: 'ISource', name: 'Current source', symbol: S.ISourceSymbol, defaults: { amps: 0.01 } },
  { id: 'diode', kind: 'Diode', name: 'Diode', symbol: S.DiodeSymbol, defaults: {} },
  { id: 'led', kind: 'Led', name: 'LED', symbol: S.LedSymbol, defaults: { max_amps: 0.02 } },
  { id: 'switch', kind: 'Switch', name: 'Switch', symbol: S.SwitchSymbol, defaults: { closed: 1 } }, // 1 = closed, 0 = open
  { id: 'ground', kind: 'Ground', name: 'Ground', symbol: S.GroundSymbol, defaults: {} },
]

const core = (k: PartKindName) => partDefs.find((d) => d.kind === k)!

const kid = (id: string, tile: KidsTile, kind: PartKindName, defaults: Record<string, number> = core(kind).defaults): PartDef => ({
  ...core(kind), id: `kids-${id}`, name: tile.name, kidsTile: tile, defaults, builds: [{ kind, defaults }],
})

export function kidsPalette(): PartDef[] {
  const motor = kid('motor', tiles.motor, 'Resistor', { ohms: 50, max_watts: 1 })
  motor.builds = [...motor.builds!, { kind: 'Inductor', defaults: { henries: 0.01 } }] // coil in series
  return [
    kid('battery', tiles.battery, 'VSource'),
    kid('bulb', tiles.bulb, 'Resistor', { ohms: 100, max_watts: 0.5 }), // 0.5 W bulb rating: burns out above it
    kid('switch', tiles.switch, 'Switch'),
    kid('buzzer', tiles.buzzer, 'Resistor', { ohms: 200, max_watts: 0.5 }),
    kid('resistor', tiles.resistor, 'Resistor'),
    motor,
  ]
}
