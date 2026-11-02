import { expect, test } from 'vitest'
import { partDefs } from '../parts/registry'
import type { CircuitJson } from '../sim/api'
import { buildCircuit, nodeLabels } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { addPart, addWire, emptySchematic, pinCount, pinPos } from './schematic'

const def = (id: string) => partDefs.find((d) => d.id === id)!
const at = { x: 100, y: 100 }

test('mosfet_has_three_pins_drain_gate_source', () => {
  const p = addPart(emptySchematic(), def('nmos'), at).parts[0]
  expect(pinCount(p)).toBe(3)
  expect([0, 1, 2].map((i) => pinPos(p, i))).toEqual([{ x: 134, y: 84 }, { x: 66, y: 100 }, { x: 134, y: 116 }])
})

test('build_circuit_maps_pins_and_polarity', () => {
  for (const [id, pmos] of [['nmos', false], ['pmos', true]] as const) {
    let s = addPart(addPart(addPart(emptySchematic(), def(id), at), 'Resistor', at), 'Ground', at)
    s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
    s = addWire(s, { part: 'p1', pin: 2 }, { part: 'p3', pin: 0 })
    expect(buildCircuit(s).parts[0]).toEqual({ id: 'p1', kind: 'Mosfet', d: 1, g: 2, s: 0, pmos, vth: 2, kp: 0.02, max_amps: 0.5 })
    expect(nodeLabels(s)[1]).toContain('p1 drain')
  }
})

test('file_roundtrip_keeps_nmos_and_pmos', () => {
  const c: CircuitJson = { version: 1, parts: [
    { id: 'M1', kind: 'Mosfet', d: 1, g: 2, s: 0, pmos: false, vth: 1.5, kp: 0.01, max_amps: 0.2 },
    { id: 'M2', kind: 'Mosfet', d: 3, g: 2, s: 0, pmos: true, vth: 2, kp: 0.02, max_amps: 0.5 },
  ] }
  const s = schematicFromCircuit(c)
  expect(s.parts.map((p) => p.def.id)).toEqual(['nmos', 'pmos', 'ground'])
  expect(buildCircuit(s)).toEqual(c)
})
