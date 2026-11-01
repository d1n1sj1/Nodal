import { expect, test } from 'vitest'
import { addPart, emptySchematic } from '../editor/schematic'
import { kidsPalette } from '../parts/registry'
import type { SimOutput } from '../sim/api'
import { glowLevels } from './glow'

const bulb = kidsPalette().find((d) => d.id === 'kids-bulb')!
const s = addPart(emptySchematic(), bulb, { x: 0, y: 0 })
const out = (i: number) => ({ times: [0, 1], node_voltages: [], part_currents: { [s.parts[0].id]: [0, i] }, warnings: [] }) as SimOutput

test('bulb_brightness_is_power_over_rating_clamped', () => {
  const { ohms, max_watts } = s.parts[0].values
  const half = Math.sqrt((0.5 * max_watts) / ohms)
  expect(glowLevels(out(half), s).get(s.parts[0].id)).toBeCloseTo(0.5)
  expect(glowLevels(out(10), s).get(s.parts[0].id)).toBe(1)
  expect(glowLevels(out(0), s).get(s.parts[0].id)).toBe(0)
  expect(glowLevels(null, s).size).toBe(0)
})
