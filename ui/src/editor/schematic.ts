import { partDefs, type PartDef } from '../parts/registry'

export interface Pos { x: number; y: number }
/** A terminal: part id + pin index (0/1; Ground has only pin 0). */
export interface Pin { part: string; pin: number }
export interface PlacedPart { id: string; def: PartDef; values: Record<string, number>; pos: Pos }
export interface Wire { from: Pin; to: Pin }
export interface Schematic { parts: PlacedPart[]; wires: Wire[] }

export const emptySchematic = (): Schematic => ({ parts: [], wires: [] })
export const pinCount = (p: PlacedPart) => (p.def.kind === 'Ground' ? 1 : 2)

export const PART_HALF_WIDTH = 34
export const PART_HALF_HEIGHT = 22
/** Canvas position of a pin: two-pin parts left/right, Ground on top. */
export const pinPos = (p: PlacedPart, i: number): Pos =>
  p.def.kind === 'Ground' ? { x: p.pos.x, y: p.pos.y - 16 } : { x: p.pos.x + (i ? PART_HALF_WIDTH : -PART_HALF_WIDTH), y: p.pos.y }

const samePin = (a: Pin, b: Pin) => a.part === b.part && a.pin === b.pin

export function addPart(s: Schematic, what: string | PartDef, pos: Pos): Schematic {
  const def = typeof what === 'string' ? partDefs.find((d) => d.kind === what) : what
  if (!def) throw new Error(`unknown part kind: ${String(what)}`)
  const ids = new Set(s.parts.map((p) => p.id))
  let n = 1
  while (ids.has(`p${n}`)) n++
  return { ...s, parts: [...s.parts, { id: `p${n}`, def, values: { ...def.defaults }, pos }] }
}

export function addPlaced(s: Schematic, part: PlacedPart): Schematic {
  return { ...s, parts: [...s.parts, part] }
}

export function addWire(s: Schematic, from: Pin, to: Pin): Schematic {
  const ok = (p: Pin) => { const part = s.parts.find((x) => x.id === p.part); return !!part && p.pin >= 0 && p.pin < pinCount(part) }
  if (samePin(from, to) || !ok(from) || !ok(to)) return s
  if (s.wires.some((w) => (samePin(w.from, from) && samePin(w.to, to)) || (samePin(w.from, to) && samePin(w.to, from)))) return s
  return { ...s, wires: [...s.wires, { from, to }] }
}

export const removePart = (s: Schematic, id: string): Schematic => ({
  parts: s.parts.filter((p) => p.id !== id),
  wires: s.wires.filter((w) => w.from.part !== id && w.to.part !== id),
})

export const removeWire = (s: Schematic, index: number): Schematic => ({ ...s, wires: s.wires.filter((_, i) => i !== index) })

export const movePart = (s: Schematic, id: string, pos: Pos): Schematic => ({
  ...s,
  parts: s.parts.map((p) => (p.id === id ? { ...p, pos } : p)),
})
