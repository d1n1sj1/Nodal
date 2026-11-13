import type { CircuitJson } from '../sim/api'
import { partDefs } from '../parts/registry'
import { PIN_FIELDS, WAVE_SHAPES } from './buildCircuit'
import { addWire, type Pin, type PlacedPart, type Schematic } from './schematic'

/** Core files carry no layout: lay parts out on a grid and rebuild wires from shared nodes. Throws on bad input. */
export function schematicFromCircuit(c: CircuitJson): Schematic {
  if (!c || !Array.isArray(c.parts)) throw new Error('This file is not a Nodal circuit.')
  if (c.version !== 1) throw new Error('This file was made by a newer version of Nodal. Update Nodal to open it.')
  const parts: PlacedPart[] = []
  const byNode = new Map<number, Pin[]>()
  c.parts.forEach((jp, i) => {
    const def = partDefs.find((d) => d.kind === jp.kind && (jp.kind === 'VSource' ? ('shape' in d.defaults) === !!jp.wave : jp.kind === 'Bjt' ? d.defaults.pnp === (jp.pnp ? 1 : 0) : jp.kind !== 'Mosfet' || d.defaults.pmos === (jp.pmos ? 1 : 0)))
    if (!def) throw new Error(`Unknown part kind "${String((jp as { kind: unknown }).kind)}".`)
    const rec = jp as unknown as Record<string, number | boolean>
    const values: Record<string, number> = {}
    for (const k of Object.keys(def.defaults)) values[k] = typeof rec[k] === 'boolean' ? (rec[k] ? 1 : 0) : (rec[k] as number)
    if (jp.kind === 'VSource' && jp.wave) Object.assign(values, { amp: jp.wave.amp, freq: jp.wave.freq, shape: WAVE_SHAPES.indexOf(jp.wave.shape), duty: jp.wave.duty ?? 0.5, phase: jp.wave.phase ?? 0 })
    parts.push({ id: jp.id, def, values, pos: { x: 120 + (i % 4) * 180, y: 80 + Math.floor(i / 4) * 120 } })
    PIN_FIELDS[def.kind as keyof typeof PIN_FIELDS].forEach((f, pin) => {
      const n = rec[f] as number
      byNode.set(n, [...(byNode.get(n) ?? []), { part: jp.id, pin }])
    })
  })
  let s: Schematic = { parts, wires: [] }
  if (byNode.has(0)) {
    const g = partDefs.find((d) => d.kind === 'Ground')!
    s = { ...s, parts: [...s.parts, { id: 'GND', def: g, values: {}, pos: { x: 120, y: 80 + Math.ceil(parts.length / 4) * 120 } }] }
    byNode.get(0)!.unshift({ part: 'GND', pin: 0 })
  }
  for (const pins of byNode.values()) for (const p of pins.slice(1)) s = addWire(s, pins[0], p)
  return s
}
