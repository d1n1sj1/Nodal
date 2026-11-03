import { useRef, useState, type MouseEvent } from 'react'
import type { Mode } from '../modes/mode'
import type { PartDef } from '../parts/registry'
import { tileFor, tiles } from '../parts/tiles'
import { routeAll } from './route'
import { addPart, addWire, movePart, pinCount, pinPos, removePart, removeWire, PART_HALF_WIDTH, PART_HALF_HEIGHT, type Pin, type Pos, type Schematic } from './schematic'

const GRID = 20
const snap = (n: number) => Math.round(n / GRID) * GRID

interface Props {
  schematic: Schematic
  onChange: (s: Schematic) => void
  mode: Mode
  /** Palette pick waiting for a canvas click. */
  placing?: PartDef | null
  onPlaced?: () => void
  /** Placed-part ids with damage warnings: drawn red. */
  damaged?: Set<string>
  /** Selected placed-part id (null when nothing or a wire is selected), for the properties panel. */
  onSelect?: (partId: string | null) => void
  /** Kids bulb brightness 0..1 by placed-part id. */
  glow?: Map<string, number>
}

export default function Canvas({ schematic, onChange, mode, placing, onPlaced, damaged, onSelect, glow }: Props) {
  const svg = useRef<SVGSVGElement>(null)
  const drag = useRef<{ id: string; dx: number; dy: number } | null>(null)
  const [selected, setSelectedState] = useState<string | null>(null) // "part:<id>" | "wire:<index>"
  const setSelected = (sel: string | null) => {
    setSelectedState(sel)
    onSelect?.(sel?.startsWith('part:') ? sel.slice(5) : null)
  }
  const [wiring, setWiring] = useState<Pin | null>(null)

  const at = (e: MouseEvent): Pos => {
    const r = svg.current!.getBoundingClientRect()
    return { x: e.clientX - r.left, y: e.clientY - r.top }
  }

  const onBackground = (e: MouseEvent) => {
    if (placing) {
      const p = at(e)
      onChange(addPart(schematic, placing, { x: snap(p.x), y: snap(p.y) }))
      onPlaced?.()
    }
    setSelected(null)
    setWiring(null)
  }

  const onPin = (e: { stopPropagation: () => void }, pin: Pin) => {
    e.stopPropagation()
    if (!wiring) return setWiring(pin)
    onChange(addWire(schematic, wiring, pin))
    setWiring(null)
  }

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.target === e.currentTarget && [' ', 'Enter', 'Delete', 'Backspace'].includes(e.key)) e.preventDefault() // no page scroll / navigation
    if ((e.key === 'Enter' || e.key === ' ') && placing && e.target === e.currentTarget) {
      // taken = bounding box (68x44 + margin) overlaps any existing part, on-grid or not
      const taken = (x: number, y: number) =>
        schematic.parts.some((p) => Math.abs(p.pos.x - x) < PART_HALF_WIDTH * 2 + 12 && Math.abs(p.pos.y - y) < PART_HALF_HEIGHT * 2 + 24)
      let k = 0 // next free cell in a 6-wide layout
      while (taken(100 + (k % 6) * 140, 100 + Math.floor(k / 6) * 100)) k++
      onChange(addPart(schematic, placing, { x: 100 + (k % 6) * 140, y: 100 + Math.floor(k / 6) * 100 }))
      onPlaced?.()
      return
    }
    if (e.key === 'Escape') return setWiring(null)
    if ((e.key !== 'Delete' && e.key !== 'Backspace') || !selected) return
    const [type, id] = selected.split(':')
    onChange(type === 'part' ? removePart(schematic, id) : removeWire(schematic, Number(id)))
    setSelected(null)
    svg.current?.focus() // the focused element is about to vanish; keep keyboard users on the canvas
  }

  const kids = mode === 'kids'
  return (
    <svg
      ref={svg}
      className={`canvas ${placing ? 'canvas-placing' : ''}`}
      role="application"
      aria-label="Circuit canvas"
      tabIndex={0}
      onClick={onBackground}
      onKeyDown={onKeyDown}
      onMouseMove={(e) => {
        const d = drag.current
        if (d) { const p = at(e); onChange(movePart(schematic, d.id, { x: snap(p.x - d.dx), y: snap(p.y - d.dy) })) }
      }}
      onMouseUp={() => (drag.current = null)}
      onMouseLeave={() => (drag.current = null)}
    >
      <defs>
        <pattern id="grid" width={GRID} height={GRID} patternUnits="userSpaceOnUse">
          <circle cx="1" cy="1" r="1" className="grid-dot" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#grid)" />

      {(() => { const routes = routeAll(schematic.parts, schematic.wires); return schematic.wires.map((w, i) => {
        const points = routes[i].map((p) => `${p.x},${p.y}`).join(' ')
        return (
          <polyline key={i} role="button" tabIndex={0} aria-label={`Wire ${w.from.part} pin ${w.from.pin + 1} to ${w.to.part} pin ${w.to.pin + 1}`}
            className={`wire ${selected === `wire:${i}` ? 'selected' : ''}`} points={points}
            onClick={(e) => { e.stopPropagation(); setSelected(`wire:${i}`) }}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); setSelected(`wire:${i}`) } }} />
        )
      }) })()}

      {schematic.parts.map((p) => {
        const tile = kids ? tileFor(p.def) : null
        const Icon = tile ? tile.picture : p.def.symbol
        return (
          <g key={p.id} data-part-id={p.id} role="group" aria-label={(p.def.kind === 'Ground' ? p.def.name : `${tile ? tile.name : p.def.name} ${p.id}`) + (damaged?.has(p.id) ? ' — warning' : '')}
            className={`placed ${damaged?.has(p.id) ? 'damaged' : ''} ${selected === `part:${p.id}` ? 'selected' : ''}`} transform={`translate(${p.pos.x} ${p.pos.y})`}
            onMouseDown={(e) => {
              e.stopPropagation()
              setSelected(`part:${p.id}`)
              const m = at(e)
              drag.current = { id: p.id, dx: m.x - p.pos.x, dy: m.y - p.pos.y }
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {damaged?.has(p.id) && <rect className="damage-halo" x={-PART_HALF_WIDTH - 4} y="-26" width={PART_HALF_WIDTH * 2 + 8} height="52" rx="12" />}
            <g role="button" tabIndex={0} aria-label={`Select ${tile ? tile.name : p.def.name} ${p.id}`}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); setSelected(`part:${p.id}`) } }}>
            {tile ? (
              <>
                <rect x={-PART_HALF_WIDTH} y="-22" width={PART_HALF_WIDTH * 2} height="44" rx="10" fill={tile.color} />
                {tile === tiles.bulb && (glow?.get(p.id) ?? 0) > 0.02 && (
                  <circle data-testid="bulb-glow" cx="0" cy="-6" r="24" fill="#fbbf24" opacity={0.25 + 0.6 * glow!.get(p.id)!} />
                )}
                <g transform="translate(-22 -22)"><Icon /></g>
              </>
            ) : (
              <g transform="translate(-34 -18) scale(2)"><Icon /></g>
            )}
            </g>
            <text y="36" textAnchor="middle" className="part-label">{tile ? tile.name : p.def.name}</text>
            {Array.from({ length: pinCount(p) }, (_, i) => {
              const c = pinPos(p, i)
              // pins join the tab order only for the selected part: one tab stop per part, pins one Tab after selecting
              const active = wiring?.part === p.id && wiring.pin === i
              return (
                <circle key={i} role="button" tabIndex={selected === `part:${p.id}` ? 0 : -1} aria-label={`${p.id} pin ${i + 1}`} className={`pin ${active ? 'active' : ''}`}
                  cx={c.x - p.pos.x} cy={c.y - p.pos.y} r="5"
                  onMouseDown={(e) => e.stopPropagation()} onClick={(e) => onPin(e, { part: p.id, pin: i })}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onPin(e, { part: p.id, pin: i }) } }} />
              )
            })}
          </g>
        )
      })}
    </svg>
  )
}
