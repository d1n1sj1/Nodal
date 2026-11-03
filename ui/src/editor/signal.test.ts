import { expect, test } from 'vitest'
import { partDefs } from '../parts/registry'
import { buildCircuit } from './buildCircuit'
import { schematicFromCircuit } from './fromCircuit'
import { addPart, addWire, emptySchematic } from './schematic'

const signal = partDefs.find((d) => d.id === 'signal')!

function circuit(square: number) {
  let s = addPart(emptySchematic(), { ...signal, defaults: { ...signal.defaults, square } }, { x: 100, y: 100 })
  s = addPart(s, 'Resistor', { x: 300, y: 100 })
  s = addPart(s, 'Ground', { x: 300, y: 200 })
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p2', pin: 1 }, { part: 'p3', pin: 0 })
  return addWire(s, { part: 'p1', pin: 1 }, { part: 'p3', pin: 0 })
}

test('signal_source_builds_a_wave_and_reloads_as_a_signal_source', () => {
  for (const [square, shape] of [[0, 'Sine'], [1, 'Square']] as const) {
    const c = buildCircuit(circuit(square))
    expect(c.parts[0]).toEqual({ id: 'p1', kind: 'VSource', pos: 1, neg: 0, volts: 0, wave: { shape, amp: 1, freq: 1000 } })
    const back = schematicFromCircuit(c).parts[0]
    expect(back.def.id).toBe('signal')
    expect(back.values).toEqual({ volts: 0, amp: 1, freq: 1000, square })
  }
})

test('plain_battery_still_has_no_wave_and_loads_as_a_voltage_source', () => {
  const c = { version: 1 as const, parts: [{ id: 'V1', kind: 'VSource' as const, pos: 1, neg: 0, volts: 5 }] }
  expect(schematicFromCircuit(c).parts[0].def.id).toBe('vsource')
})
