import { expect, test } from 'vitest'
import { kidsPalette } from '../parts/registry'
import type { CircuitJson } from '../sim/api'
import { buildCircuit, nodeLabels } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { addPart, addWire, emptySchematic, updateValues, type Schematic } from './schematic'

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
  expect(() => schematicFromCircuit({ version: 2, parts: [] } as never)).toThrow(/newer version of Nodal/)
  expect(() => schematicFromCircuit({ version: 1, parts: [{ id: 'x', kind: 'Bogus' }] } as never)).toThrow(/Unknown part/)
})

const kid = (id: string) => kidsPalette().find((d) => d.id === `kids-${id}`)!

/** Kids battery (p1) + bulb (p2) in a loop, no Ground part. */
function kidsLoop(): Schematic {
  let s = addPart(addPart(emptySchematic(), kid('battery'), at), kid('bulb'), at)
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  return addWire(s, { part: 'p2', pin: 1 }, { part: 'p1', pin: 1 })
}

test('no_ground_part_grounds_first_vsource_neg', () => {
  const [v, r] = buildCircuit(kidsLoop()).parts as unknown as { neg: number; pos: number; a: number; b: number }[]
  expect(v.neg).toBe(0)
  expect(r.b).toBe(0)
  expect(v.pos).toBe(r.a)
  expect(v.pos).not.toBe(0)
})

test('no_ground_and_no_vsource_grounds_first_isource_neg', () => {
  let s = addPart(addPart(emptySchematic(), 'Resistor', at), 'ISource', at)
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p2', pin: 1 })
  expect((buildCircuit(s).parts[1] as { to: number }).to).toBe(0)
})

test('edited_values_flow_into_build_circuit_and_ids_stay_stable', () => {
  const s = updateValues(divider(), 'p2', { ohms: 2200 })
  const c = buildCircuit(s)
  expect(c.parts[1]).toMatchObject({ id: 'p2', ohms: 2200, max_watts: 0.25 })
  expect(buildCircuit(divider()).parts[1]).toMatchObject({ ohms: 1000 }) // immutable
  const k = buildCircuit(updateValues(updateValues(kidsLoop(), 'p1', { volts: 3 }), 'p2', { ohms: 47 }))
  expect(k.parts.map((p) => p.id)).toEqual(['p1', 'p2'])
  expect(k.parts[0]).toMatchObject({ volts: 3 })
  expect(k.parts[1]).toMatchObject({ ohms: 47 })
})

test('toggled_switch_builds_closed_false', () => {
  const s = updateValues(addPart(emptySchematic(), kid('switch'), at), 'p1', { closed: 0 })
  expect((buildCircuit(s).parts[0] as { closed: unknown }).closed).toBe(false)
})

test('kids_led_transistor_amplifier_build_core_parts', () => {
  let s = addPart(addPart(addPart(emptySchematic(), kid('led'), at), kid('transistor'), at), kid('amplifier'), at)
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 }) // LED cathode -> collector
  s = addWire(s, { part: 'p2', pin: 2 }, { part: 'p3', pin: 0 }) // emitter -> amp output
  const [led, bjt, amp] = buildCircuit(s).parts as unknown as Record<string, unknown>[]
  expect(led).toMatchObject({ kind: 'Led', max_amps: 0.02 })
  expect(bjt).toMatchObject({ kind: 'Bjt', pnp: false, beta: 100 })
  expect(amp).toMatchObject({ kind: 'Opamp', vsat: 12 })
  expect(led.cathode).toBe(bjt.c)
  expect(bjt.e).toBe(amp.out)
  expect(new Set([bjt.c, bjt.b, bjt.e]).size).toBe(3) // unwired pins get their own nodes
})

test('kids_capacitor_builds_a_core_capacitor', () => {
  const s = addWire(addPart(addPart(emptySchematic(), kid('capacitor'), at), kid('resistor'), at), { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  const [cap, r] = buildCircuit(s).parts as unknown as { kind: string; farads: number; b: number; a: number }[]
  expect(cap).toMatchObject({ kind: 'Capacitor', farads: 1e-6 })
  expect(cap.b).toBe(r.a)
})

test('kids_motor_keeps_builds_defaults', () => {
  const s = updateValues(addPart(emptySchematic(), kid('motor'), at), 'p1', { ohms: 1 })
  expect(buildCircuit(s).parts[0]).toMatchObject({ ohms: 50 })
})

test('node_labels_name_parts_on_each_node', () => {
  expect(nodeLabels(divider())).toEqual([
    '0: ground',
    '1: Voltage source p1 left, Resistor p2 left',
    '2: Resistor p2 right, Resistor p3 left',
  ])
})

test('node_labels_match_build_numbering_for_kids_parts', () => {
  let s = addPart(kidsLoop(), kid('motor'), at) // p3, unwired
  s = addWire(s, { part: 'p3', pin: 0 }, { part: 'p1', pin: 0 })
  const c = buildCircuit(s)
  const labels = nodeLabels(s)
  const [r, l] = c.parts.slice(2) as { a: number; b: number }[]
  expect(labels[0]).toBe('0: ground')
  expect(labels[r.a]).toContain('Motor p3 left')
  expect(labels[r.a]).toContain('Battery p1 left')
  expect(labels[r.b]).toBe(`${r.b}: Motor p3 inside`)
  expect(labels[l.b]).toBe(`${l.b}: Motor p3 right`)
  expect(labels).toHaveLength(l.b + 1)
})
