import type { Mode } from '../modes/mode'
import { kidsPalette, partDefs, type PartDef } from '../parts/registry'
import { tileFor } from '../parts/tiles'

export default function Palette({ mode, onPick }: { mode: Mode; onPick: (def: PartDef) => void }) {
  const kids = mode === 'kids'
  return (
    <nav className={`palette ${kids ? 'palette-kids' : ''}`} aria-label="Parts">
      {(kids ? kidsPalette() : partDefs).map((def) => {
        const tile = kids ? tileFor(def) : null
        const Icon = tile ? tile.picture : def.symbol
        return (
          <button key={def.id} type="button" className={kids ? 'tile' : 'part'} style={tile ? { background: tile.color } : undefined} onClick={() => onPick(def)}>
            <Icon />
            <span>{tile ? tile.name : def.name}</span>
          </button>
        )
      })}
    </nav>
  )
}
