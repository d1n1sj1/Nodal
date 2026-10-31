import { expect, test } from 'vitest'
import { kidsPalette } from '../parts/registry'
import { addPart, addWire, emptySchematic, movePart, removePart } from './schematic'

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
