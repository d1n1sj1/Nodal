import { useSyncExternalStore } from 'react'

export type Mode = 'engineer' | 'kids'
const KEY = 'nodal.mode'

function read(): Mode {
  try {
    const v = localStorage.getItem(KEY)
    return v === 'kids' ? 'kids' : 'engineer'
  } catch {
    return 'engineer'
  }
}

let current: Mode = read()
const listeners = new Set<() => void>()
const apply = () => document.documentElement.setAttribute('data-mode', current)
apply()

export function setMode(m: Mode) {
  current = m
  try {
    localStorage.setItem(KEY, m)
  } catch {
    /* storage unavailable: mode stays in memory only */
  }
  apply()
  listeners.forEach((l) => l())
}

const subscribe = (l: () => void) => {
  listeners.add(l)
  return () => void listeners.delete(l)
}

export function useMode() {
  const mode = useSyncExternalStore(subscribe, () => current)
  return { mode, setMode }
}
