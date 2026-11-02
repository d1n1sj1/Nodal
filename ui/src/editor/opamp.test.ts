import { expect, test } from 'vitest'
import type { CircuitJson } from '../sim/api'
import { buildCircuit, nodeLabels } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { routeWire } from './route'
import { addPart, addWire, emptySchematic, pinCount, pinPos } from './schematic'

const at = { x: 100, y: 100 }

test('opamp_pins_are_output_right_inputs_left', () => {
  const s = addPart(addPart(emptySchematic(), 'Opamp', at), 'Resistor', { x: 300, y: 100 })
  const p = s.parts[0]
  expect(pinCount(p)).toBe(3)
  expect([0, 1, 2].map((i) => pinPos(p, i))).toEqual([{ x: 134, y: 100 }, { x: 66, y: 88 }, { x: 66, y: 112 }])
  const w = routeWire(s.parts, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  expect(w[1].x).toBeLessThan(w[0].x) // inputs leave to the left
})

test('build_circuit_maps_pins_and_defaults', () => {
  let s = addPart(addPart(addPart(emptySchematic(), 'Opamp', at), 'Resistor', at), 'Ground', at)
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p3', pin: 0 })
  expect(buildCircuit(s).parts[0]).toEqual({ id: 'p1', kind: 'Opamp', out: 1, plus: 0, minus: 2, gain: 1e5, vsat: 12, max_amps: 0.025 })
  expect(nodeLabels(s)[2]).toContain('p1 inverting input')
})

test('file_roundtrip_keeps_opamp', () => {
  const c: CircuitJson = { version: 1, parts: [{ id: 'U1', kind: 'Opamp', out: 1, plus: 0, minus: 2, gain: 5e4, vsat: 5, max_amps: 0.01 }] }
  expect(buildCircuit(schematicFromCircuit(c))).toEqual(c)
})
