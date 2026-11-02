import type { CircuitJson, PartJson } from '../sim/api'
import type { PartKindName } from '../parts/registry'
import { pinCount, type Pin, type Schematic } from './schematic'

/** Core field names of the terminals, in pin order, per kind (match nodal-core model.rs). */
export const PIN_FIELDS: Record<PartKindName, string[]> = {
  Resistor: ['a', 'b'], Capacitor: ['a', 'b'], Inductor: ['a', 'b'], Switch: ['a', 'b'],
  VSource: ['pos', 'neg'], ISource: ['from', 'to'], Diode: ['anode', 'cathode'], Led: ['anode', 'cathode'],
  Bjt: ['c', 'b', 'e'],
}

export const pinKey = (p: Pin) => `${p.part}:${p.pin}`

/** Shared by buildCircuit and nodeLabels so node numbers always agree. `pins` maps every pin/mid key nodeOf saw to its node. */
function solve(s: Schematic): { parts: PartJson[]; pins: Map<string, number> } {
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
  const grounds = s.parts.filter((p) => p.def.kind === 'Ground')
  for (const p of grounds) for (let i = 0; i < pinCount(p); i++) union(pinKey({ part: p.id, pin: i }), 'GND')
  // Kids mode has no Ground tile: without a reference the core reports NoGround, so ground the first source's negative pin.
  const src = grounds.length ? undefined : s.parts.find((p) => p.def.kind === 'VSource') ?? s.parts.find((p) => p.def.kind === 'ISource')
  if (src) union(pinKey({ part: src.id, pin: 1 }), 'GND')

  const ids = new Map<string, number>([[find('GND'), 0]])
  const pins = new Map<string, number>()
  const nodeOf = (key: string) => {
    const r = find(key) // an unwired pin is its own set, so it gets a fresh node (core reports FloatingNode)
    if (!ids.has(r)) ids.set(r, ids.size)
    pins.set(key, ids.get(r)!)
    return ids.get(r)!
  }

  const parts: PartJson[] = []
  for (const p of s.parts) {
    if (p.def.kind === 'Ground') continue
    // single-build kids parts honour edited values; multi-build parts (motor) keep their builds defaults
    const builds = p.def.builds && p.def.builds.length > 1 ? p.def.builds : [{ kind: (p.def.builds?.[0].kind ?? p.def.kind) as PartKindName, defaults: p.values }]
    // Series chain: pin0 - build0 - mid1 - build1 - ... - pin1
    const chain = [`${p.id}:0`, ...builds.slice(1).map((_, i) => `${p.id}:mid${i + 1}`), `${p.id}:1`]
    builds.forEach((b, i) => {
      const fields = PIN_FIELDS[b.kind]
      const vals: Record<string, number | boolean> = { ...b.defaults }
      if (b.kind === 'Switch') vals.closed = vals.closed !== 0
      if (b.kind === 'Bjt') vals.pnp = vals.pnp !== 0
      const id = builds.length > 1 ? `${p.id}-${b.kind.toLowerCase()}` : p.id
      // a 3-pin part wires each field straight to its own pin; two-pin builds share the series chain
      const keys = fields.length === 3 ? fields.map((_, k) => `${p.id}:${k}`) : [chain[i], chain[i + 1]]
      parts.push({ id, kind: b.kind, ...Object.fromEntries(fields.map((f, k) => [f, nodeOf(keys[k])])), ...vals } as PartJson)
    })
  }
  return { parts, pins }
}

export const buildCircuit = (s: Schematic): CircuitJson => ({ version: 1, parts: solve(s).parts })

const SIDE: Record<string, string> = { 0: 'left', 1: 'right' }
const BJT_PINS = ['collector', 'base', 'emitter']

/** One label per core node, e.g. "2: Resistor p2 right, Resistor p3 left"; node 0 is "0: ground". */
export function nodeLabels(s: Schematic): string[] {
  const names: string[][] = []
  for (const [key, n] of solve(s).pins) {
    const cut = key.lastIndexOf(':') // loaded ids may contain ':'
    const [id, pin] = [key.slice(0, cut), key.slice(cut + 1)]
    const p = s.parts.find((x) => x.id === id)!
    ;(names[n] ??= []).push(`${p.def.name} ${id} ${p.def.kind === 'Bjt' ? BJT_PINS[Number(pin)] : SIDE[pin] ?? 'inside'}`)
  }
  return Array.from({ length: names.length || 1 }, (_, n) => (n === 0 ? '0: ground' : `${n}: ${names[n].join(', ')}`))
}
