import { expect, test } from 'vitest'
import { kidsPalette } from '../parts/registry'
import type { CircuitJson } from '../sim/api'
import { buildCircuit } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { addPart, addWire, emptySchematic, type Schematic } from './schematic'

const at = { x: 0, y: 0 }
/** V(p1) - R1(p2) - R2(p3) - back to V, ground (p4) on V.neg. */
function divider(): Schematic {
  let s = emptySchematic()
  for (const k of ['VSource', 'Resistor', 'Resistor', 'Ground']) s = addPart(s, k, at)
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p2', pin: 1 }, { part: 'p3', pin: 0 })
  s = addWire(s, { part: 'p3', pin: 1 }, { part: 'p1', pin: 1 })
  return addWire(s, { part: 'p1', pin: 1 }, { part: 'p4', pin: 0 })
}

test('wired_pins_share_a_node', () => {
  let s = addPart(addPart(emptySchematic(), 'Resistor', at), 'Resistor', at)
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  const [r1, r2] = buildCircuit(s).parts as { a: number; b: number }[]
  expect(r1.b).toBe(r2.a)
  expect(new Set([r1.a, r1.b, r2.b]).size).toBe(3) // unwired pins stay separate
})

test('ground_part_maps_to_node_zero', () => {
  let s = addPart(addPart(emptySchematic(), 'Resistor', at), 'Ground', at)
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  const c = buildCircuit(s)
  expect(c.parts).toHaveLength(1) // ground yields no core part
  expect((c.parts[0] as { b: number }).b).toBe(0)
  expect((c.parts[0] as { a: number }).a).toBe(1)
})

test('build_circuit_of_divider_has_expected_parts_and_nodes', () => {
  const expected: CircuitJson = {
    version: 1,
    parts: [
      { id: 'p1', kind: 'VSource', pos: 1, neg: 0, volts: 5 },
      { id: 'p2', kind: 'Resistor', a: 1, b: 2, ohms: 1000, max_watts: 0.25 },
      { id: 'p3', kind: 'Resistor', a: 2, b: 0, ohms: 1000, max_watts: 0.25 },
    ],
  }
  expect(buildCircuit(divider())).toEqual(expected)
  expect(JSON.stringify(buildCircuit(divider()))).not.toContain('"mode"')
})

test('empty_schematic_builds_empty_circuit', () => {
  expect(buildCircuit(emptySchematic())).toEqual({ version: 1, parts: [] })
})

test('switch_closed_is_boolean', () => {
  const p = buildCircuit(addPart(emptySchematic(), 'Switch', at)).parts[0] as { closed: unknown }
  expect(p.closed).toBe(true)
})

test('kids_motor_builds_resistor_and_inductor_in_series', () => {
  const motor = kidsPalette().find((d) => d.id === 'kids-motor')!
  const c = buildCircuit(addPart(emptySchematic(), motor, at))
  const [r, l] = c.parts as { id: string; kind: string; a: number; b: number }[]
  expect([r.kind, l.kind]).toEqual(['Resistor', 'Inductor'])
  expect(r.id).not.toBe(l.id)
  expect(r.b).toBe(l.a) // internal node between them
  expect(new Set([r.a, r.b, l.b]).size).toBe(3)
})

test('from_circuit_round_trips_divider', () => {
  const c = buildCircuit(divider())
  expect(buildCircuit(schematicFromCircuit(c))).toEqual(c)
})

test('from_circuit_rejects_garbage', () => {
  expect(() => schematicFromCircuit({ version: 2, parts: [] } as never)).toThrow(/not a Nodal circuit/)
  expect(() => schematicFromCircuit({ version: 1, parts: [{ id: 'x', kind: 'Bogus' }] } as never)).toThrow(/Unknown part/)
})
