import { expect, test } from 'vitest'
import { routeWire } from './route'
import { addPart, emptySchematic, PART_HALF_HEIGHT, PART_HALF_WIDTH, type PlacedPart, type Pos } from './schematic'

const place = (...at: Pos[]) => at.reduce((s, p) => addPart(s, 'Resistor', p), emptySchematic())

/** True when any segment passes through the part's box (pins on the edge do not count). */
const crosses = (pts: Pos[], p: PlacedPart) =>
  pts.some((q, i) => i > 0 && Math.max(q.x, pts[i - 1].x) > p.pos.x - PART_HALF_WIDTH + 2 && Math.min(q.x, pts[i - 1].x) < p.pos.x + PART_HALF_WIDTH - 2
    && Math.max(q.y, pts[i - 1].y) > p.pos.y - PART_HALF_HEIGHT + 2 && Math.min(q.y, pts[i - 1].y) < p.pos.y + PART_HALF_HEIGHT - 2)

test('wire_goes_around_a_part_in_its_way', () => {
  const s = place({ x: 100, y: 100 }, { x: 300, y: 100 }, { x: 200, y: 100 }) // p3 sits between p1 and p2
  const pts = routeWire(s.parts, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  expect(crosses(pts, s.parts[2])).toBe(false)
  expect(pts[0]).toEqual({ x: 134, y: 100 })
  expect(pts[pts.length - 1]).toEqual({ x: 266, y: 100 })
})

test('open_route_stays_the_plain_three_segment_wire', () => {
  const s = place({ x: 100, y: 100 }, { x: 300, y: 200 })
  const pts = routeWire(s.parts, { part: 'p1', pin: 1 }, { part: 'p2', pin: 0 })
  expect(pts).toHaveLength(4)
  expect(pts[1].x).toBe(pts[2].x)
})

test('wire_never_crosses_its_own_part', () => {
  const s = place({ x: 100, y: 100 }, { x: 300, y: 100 })
  const pts = routeWire(s.parts, { part: 'p1', pin: 0 }, { part: 'p2', pin: 1 }) // left pin to right pin
  for (const p of s.parts) expect(crosses(pts, p)).toBe(false)
})
