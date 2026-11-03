import { expect, test } from 'vitest'
import { kidsPalette, partDefs, type PartDef } from './registry'
import { tileFor } from './tiles'

test('every_part_has_name_and_symbol', () => {
  expect(partDefs).toHaveLength(15)
  for (const d of partDefs) {
    expect(d.name).toBeTruthy()
    expect(typeof d.symbol).toBe('function')
  }
})

test('kids_palette_maps_to_core_kinds', () => {
  const k = Object.fromEntries(kidsPalette().map((d) => [d.name, d]))
  expect(k.Battery.kind).toBe('VSource')
  expect(k.Bulb.kind).toBe('Resistor')
  expect(k.Bulb.defaults.max_watts).toBeGreaterThan(0)
  expect(k.Switch.kind).toBe('Switch')
  expect(k.Buzzer.kind).toBe('Resistor')
  expect(k.Resistor.kind).toBe('Resistor')
  expect(k.Motor.builds?.map((b) => b.kind)).toEqual(['Resistor', 'Inductor'])
  expect(new Set(kidsPalette().map((d) => d.id)).size).toBe(6)
})

test('part_without_tile_gets_generic_tile', () => {
  const def: PartDef = { ...partDefs[0], kidsTile: undefined }
  const t = tileFor(def)
  expect(t.name).toBe(def.name)
  expect(typeof t.picture).toBe('function')
})

test('active_parts_get_own_kids_tile', () => {
  for (const id of ['npn', 'pnp', 'nmos', 'pmos', 'opamp']) {
    expect(tileFor(partDefs.find((p) => p.id === id)!).color).not.toBe('#e2e8f0')
  }
})
