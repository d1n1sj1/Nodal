import { expect, test } from 'vitest'
import type { Schematic } from '../editor/schematic'
import { addPart, emptySchematic } from '../editor/schematic'
import { damagedIds, partNameFor, warningText } from './warningText'

const led = { part_id: 'p1', kind: 'OverCurrent' as const, peak: 0.047, limit: 0.02 }

test('engineer_text_states_value_and_limit', () => {
  const t = warningText(led, 'engineer', 'LED')
  expect(t).toContain('47 mA')
  expect(t).toContain('20 mA')
  expect(warningText({ ...led, kind: 'OverPower', peak: 1.5, limit: 0.25 }, 'engineer', 'R1')).toContain('1.5 W')
  expect(warningText({ ...led, peak: Number.MAX_VALUE }, 'engineer', 'LED')).toContain('far above')
})

test('kids_text_is_friendly_and_names_the_part', () => {
  for (const kind of ['OverCurrent', 'OverPower'] as const) {
    const t = warningText({ ...led, kind }, 'kids', 'Bulb')
    expect(t).toContain('Bulb')
    expect(t).toContain('burn')
  }
})

test('partNameFor_resolves_placed_and_expanded_ids', () => {
  const s: Schematic = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  const name = s.parts[0].def.name
  expect(partNameFor(s, 'p1')).toBe(name)
  expect(partNameFor(s, 'p1-resistor')).toBe(name)
  expect(partNameFor(s, 'p1-inductor')).toBe(name)
  expect(partNameFor(s, 'zz')).toBe('zz')
})

test('exact_placed_id_wins_over_expanded_suffix', () => {
  // a reloaded kids motor is two plain parts named p1-resistor / p1-inductor
  let s: Schematic = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  s = { ...s, parts: [{ ...s.parts[0], id: 'p1-resistor' }] }
  expect(partNameFor(s, 'p1-resistor')).toBe(s.parts[0].def.name)
  expect([...damagedIds([{ ...led, part_id: 'p1-resistor' }], s)]).toEqual(['p1-resistor'])
  expect([...damagedIds([{ ...led, part_id: 'p9-inductor' }], s)]).toEqual(['p9'])
})
