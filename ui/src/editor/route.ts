import { PART_HALF_HEIGHT, PART_HALF_WIDTH, pinDir, pinPos, type PlacedPart, type Pos } from './schematic'

const GRID = 20
const INSET = 2 // pins sit on the part box edge; shrink the box so they are not "inside" it
const MAX_STEPS = 40
const snap = (n: number) => Math.round(n / GRID) * GRID

interface Rect { x0: number; y0: number; x1: number; y1: number }
const boxOf = (p: PlacedPart): Rect => ({
  x0: p.pos.x - PART_HALF_WIDTH + INSET, x1: p.pos.x + PART_HALF_WIDTH - INSET,
  y0: p.pos.y - (p.def.kind === 'Ground' ? 14 : PART_HALF_HEIGHT - INSET), y1: p.pos.y + PART_HALF_HEIGHT - INSET, // Ground's pin is above y0
})

const crosses = (a: Pos, b: Pos, r: Rect) =>
  Math.max(a.x, b.x) > r.x0 && Math.min(a.x, b.x) < r.x1 && Math.max(a.y, b.y) > r.y0 && Math.min(a.y, b.y) < r.y1

const length = (pts: Pos[]) => pts.reduce((n, p, i) => (i ? n + Math.abs(p.x - pts[i - 1].x) + Math.abs(p.y - pts[i - 1].y) : 0), 0)

/** Offsets 0, +1, -1, +2, -2, ... in grid steps. */
const spread = (centre: number) => Array.from({ length: 2 * MAX_STEPS + 1 }, (_, k) => centre + (k % 2 ? 1 : -1) * Math.ceil(k / 2) * GRID * (k ? 1 : 0))

/**
 * Right-angle wire from one pin to another. Leaves each pin outward (Ground: straight up) so it never doubles back
 * across its own part, then takes the shortest of a vertical-channel or horizontal-channel route that clears every
 * part. Falls back to the plain vertical-channel route when none clears. Does not avoid other wires.
 */
export function routeWire(parts: PlacedPart[], from: { part: string; pin: number }, to: { part: string; pin: number }): Pos[] {
  const a = parts.find((p) => p.id === from.part)!
  const b = parts.find((p) => p.id === to.part)!
  const pa = pinPos(a, from.pin)
  const pb = pinPos(b, to.pin)
  const da = pinDir(a, from.pin)
  const db = pinDir(b, to.pin)
  const ea = pa.x + da * GRID
  const eb = pb.x + db * GRID
  const boxes = parts.map(boxOf)
  const clean = (pts: Pos[]) => pts.filter((p, i) => !i || p.x !== pts[i - 1].x || p.y !== pts[i - 1].y)
  const clear = (pts: Pos[]) => pts.every((p, i) => !i || !boxes.some((r) => crosses(pts[i - 1], p, r)))

  const vertical = (mx: number): Pos[] => [pa, { x: mx, y: pa.y }, { x: mx, y: pb.y }, pb]
  const horizontal = (cy: number): Pos[] => clean([pa, { x: ea, y: pa.y }, { x: ea, y: cy }, { x: eb, y: cy }, { x: eb, y: pb.y }, pb])

  let mid = snap((ea + eb) / 2) // out, across, up/down, across, in
  if (da) mid = da > 0 ? Math.max(mid, ea) : Math.min(mid, ea)
  if (db) mid = db > 0 ? Math.max(mid, eb) : Math.min(mid, eb)
  const fallback = vertical(mid)

  const channels = spread(mid).filter((x) => (!da || (da > 0 ? x >= ea : x <= ea)) && (!db || (db > 0 ? x >= eb : x <= eb)))
  const candidates = [...channels.map(vertical), ...spread(snap((pa.y + pb.y) / 2)).map(horizontal)].filter(clear)
  return candidates.reduce((best, c) => (length(c) < length(best) ? c : best), candidates[0] ?? fallback)
}
