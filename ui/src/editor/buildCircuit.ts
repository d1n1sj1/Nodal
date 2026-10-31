import type { CircuitJson, PartJson } from '../sim/api'
import type { PartKindName } from '../parts/registry'
import { pinCount, type Pin, type Schematic } from './schematic'

/** Core field names of the two terminals, per kind (match nodal-core model.rs). */
export const PIN_FIELDS: Record<PartKindName, [string, string]> = {
  Resistor: ['a', 'b'], Capacitor: ['a', 'b'], Inductor: ['a', 'b'], Switch: ['a', 'b'],
  VSource: ['pos', 'neg'], ISource: ['from', 'to'], Diode: ['anode', 'cathode'], Led: ['anode', 'cathode'],
}

export const pinKey = (p: Pin) => `${p.part}:${p.pin}`

export function buildCircuit(s: Schematic): CircuitJson {
  const parent = new Map<string, string>()
  const find = (k: string): string => {
    if (!parent.has(k)) parent.set(k, k)
    let r = k
    while (parent.get(r) !== r) r = parent.get(r)!
    parent.set(k, r)
    return r
  }
  const union = (a: string, b: string) => parent.set(find(a), find(b))

  for (const w of s.wires) union(pinKey(w.from), pinKey(w.to))
  for (const p of s.parts) if (p.def.kind === 'Ground') for (let i = 0; i < pinCount(p); i++) union(pinKey({ part: p.id, pin: i }), 'GND')

  const ids = new Map<string, number>([[find('GND'), 0]])
  const nodeOf = (key: string) => {
    const r = find(key) // an unwired pin is its own set, so it gets a fresh node (core reports FloatingNode)
    if (!ids.has(r)) ids.set(r, ids.size)
    return ids.get(r)!
  }

  const parts: PartJson[] = []
  for (const p of s.parts) {
    if (p.def.kind === 'Ground') continue
    const builds = p.def.builds ?? [{ kind: p.def.kind, defaults: p.values }]
    // Series chain: pin0 - build0 - mid1 - build1 - ... - pin1
    const chain = [`${p.id}:0`, ...builds.slice(1).map((_, i) => `${p.id}:mid${i + 1}`), `${p.id}:1`]
    builds.forEach((b, i) => {
      const [fa, fb] = PIN_FIELDS[b.kind]
      const vals: Record<string, number | boolean> = { ...b.defaults }
      if (b.kind === 'Switch') vals.closed = vals.closed !== 0
      const id = p.def.builds && builds.length > 1 ? `${p.id}-${b.kind.toLowerCase()}` : p.id
      parts.push({ id, kind: b.kind, [fa]: nodeOf(chain[i]), [fb]: nodeOf(chain[i + 1]), ...vals } as PartJson)
    })
  }
  return { version: 1, parts }
}
