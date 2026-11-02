import { expect, test } from 'vitest'
import { partDefs } from '../parts/registry'
import type { CircuitJson } from '../sim/api'
import { buildCircuit, nodeLabels } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { routeWire } from './route'
import { addPart, addWire, emptySchematic, pinCount, pinPos, type Schematic } from './schematic'

const at = { x: 100, y: 100 }
const def = (id: string) => partDefs.find((d) => d.id === id)!

/** Q1 (p1) with its collector wired to R1 (p2); a ground on the emitter. */
function stage(q: string): Schematic {
  let s = addPart(emptySchematic(), def(q), at)
  s = addPart(s, 'Resistor', { x: 300, y: 100 })
  s = addPart(s, 'Ground', { x: 300, y: 200 })
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  return addWire(s, { part: 'p1', pin: 2 }, { part: 'p3', pin: 0 })
}

test('bjt_has_three_pins_collector_base_emitter', () => {
  const p = addPart(emptySchematic(), def('npn'), at).parts[0]
  expect(pinCount(p)).toBe(3)
  expect([0, 1, 2].map((i) => pinPos(p, i))).toEqual([{ x: 134, y: 84 }, { x: 66, y: 100 }, { x: 134, y: 116 }])
})

test('build_circuit_maps_pins_and_polarity', () => {
  for (const [q, pnp] of [['npn', false], ['pnp', true]] as const) {
    const [bjt] = buildCircuit(stage(q)).parts
    expect(bjt).toEqual({ id: 'p1', kind: 'Bjt', c: 1, b: 2, e: 0, pnp, beta: 100, max_amps: 0.1 })
  }
})

test('file_roundtrip_keeps_npn_and_pnp', () => {
  const c: CircuitJson = { version: 1, parts: [
    { id: 'Q1', kind: 'Bjt', c: 1, b: 2, e: 0, pnp: false, beta: 150, max_amps: 0.2 },
    { id: 'Q2', kind: 'Bjt', c: 3, b: 2, e: 0, pnp: true, beta: 80, max_amps: 0.05 },
  ] }
  const s = schematicFromCircuit(c)
  expect(s.parts.map((p) => p.def.id)).toEqual(['npn', 'pnp', 'ground'])
  expect(buildCircuit(s)).toEqual(c)
})

test('node_labels_name_bjt_pins', () => {
  expect(nodeLabels(stage('npn'))[1]).toContain('NPN transistor p1 collector')
})

test('wires_leave_bjt_pins_outward', () => {
  const s = stage('npn')
  const [base] = [{ part: 'p1', pin: 1 }]
  const w = routeWire(s.parts, base, { part: 'p2', pin: 0 })
  expect(w[1].x).toBeLessThan(w[0].x) // base is on the left
  const c = routeWire(s.parts, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  expect(c[1].x).toBeGreaterThan(c[0].x) // collector is on the right
})
