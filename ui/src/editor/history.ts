import { useCallback, useRef, useState } from 'react'

const LIMIT = 100
// shortcut: edits within this window merge into one undo step (drag moves, typing); a drag that pauses longer splits in two.
const MERGE_MS = 500

interface H<T> { past: T[]; present: T; future: T[] }

/** useState with undo/redo. Setter accepts a value or an updater, like useState. */
export function useHistory<T>(initial: T) {
  const [h, setH] = useState<H<T>>({ past: [], present: initial, future: [] })
  const last = useRef(0)

  const set = useCallback((next: T | ((prev: T) => T)) => {
    const now = Date.now()
    const merge = now - last.current < MERGE_MS
    last.current = now
    setH((s) => {
      const present = typeof next === 'function' ? (next as (p: T) => T)(s.present) : next
      if (present === s.present) return s
      const past = merge && s.past.length ? s.past : [...s.past, s.present].slice(-LIMIT)
      return { past, present, future: [] }
    })
  }, [])

  const undo = useCallback(() => {
    last.current = 0
    setH((s) => (s.past.length ? { past: s.past.slice(0, -1), present: s.past[s.past.length - 1], future: [s.present, ...s.future] } : s))
  }, [])
  const redo = useCallback(() => {
    last.current = 0
    setH((s) => (s.future.length ? { past: [...s.past, s.present], present: s.future[0], future: s.future.slice(1) } : s))
  }, [])

  return { state: h.present, set, undo, redo, canUndo: h.past.length > 0, canRedo: h.future.length > 0 }
}
