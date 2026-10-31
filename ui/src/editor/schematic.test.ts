import { expect, test } from 'vitest'
import { kidsPalette } from '../parts/registry'
import { buildCircuit } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { addPart, addWire, emptySchematic, movePart, removePart, updateValues } from './schematic'

test('removing_a_part_removes_its_wires', () => {
  let s = addPart(addPart(addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 }), 'Resistor', { x: 100, y: 0 }), 'Resistor', { x: 200, y: 0 })
  s = addWire(s, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p2', pin: 1 }, { part: 'p3', pin: 0 })
  const r = removePart(s, 'p2')
  expect(r.parts.map((p) => p.id)).toEqual(['p1', 'p3'])
  expect(r.wires).toEqual([])
  expect(s.parts).toHaveLength(3) // immutable
})

test('ids_are_unique_defaults_copied_and_move_is_immutable', () => {
  let s = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  s = addPart(s, 'Resistor', { x: 0, y: 0 })
  s = removePart(s, 'p1')
  s = addPart(s, kidsPalette()[0], { x: 0, y: 0 })
  expect(new Set(s.parts.map((p) => p.id)).size).toBe(2)
  s.parts[0].values.ohms = 1
  expect(addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 }).parts[0].values.ohms).toBe(1000)
  const m = movePart(s, 'p2', { x: 5, y: 6 })
  expect(m.parts[0].pos).toEqual({ x: 5, y: 6 })
  expect(s.parts[0].pos).toEqual({ x: 0, y: 0 })
})

test('bad_wires_are_ignored', () => {
  const s = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  expect(addWire(s, { part: 'p1', pin: 0 }, { part: 'p1', pin: 0 }).wires).toEqual([])
  expect(addWire(s, { part: 'p1', pin: 0 }, { part: 'nope', pin: 0 }).wires).toEqual([])
})

test('add_part_after_reloading_kids_motor_keeps_core_ids_unique', () => {
  const motor = kidsPalette().find((d) => d.id === 'kids-motor')!
  const saved = buildCircuit(addPart(emptySchematic(), motor, { x: 0, y: 0 })) // ids p1-resistor, p1-inductor
  const s = addPart(schematicFromCircuit(saved), motor, { x: 0, y: 0 })
  const ids = buildCircuit(s).parts.map((p) => p.id)
  expect(new Set(ids).size).toBe(ids.length)
  expect(s.parts.at(-1)!.id).toBe('p2')
})

test('update_values_merges_and_is_immutable', () => {
  const s = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  const u = updateValues(s, 'p1', { ohms: 5 })
  expect(u.parts[0].values).toEqual({ ohms: 5, max_watts: 0.25 })
  expect(s.parts[0].values.ohms).toBe(1000)
})
