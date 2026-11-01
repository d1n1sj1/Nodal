// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'
import { useHistory } from './history'

beforeEach(() => { vi.useFakeTimers(); vi.setSystemTime(1_000_000) })
afterEach(() => vi.useRealTimers())

const step = (ms = 1000) => vi.setSystemTime(Date.now() + ms)

test('undo_and_redo_walk_the_steps', () => {
  const { result } = renderHook(() => useHistory(0))
  act(() => result.current.set(1)); step()
  act(() => result.current.set(2))
  expect(result.current.state).toBe(2)
  act(() => result.current.undo())
  expect(result.current.state).toBe(1)
  act(() => result.current.undo())
  expect(result.current.state).toBe(0)
  expect(result.current.canUndo).toBe(false)
  act(() => result.current.redo())
  expect(result.current.state).toBe(1)
  expect(result.current.canRedo).toBe(true)
})

test('rapid_edits_merge_into_one_step', () => {
  const { result } = renderHook(() => useHistory(0))
  step()
  act(() => result.current.set(1)); step(50)
  act(() => result.current.set(2)); step(50)
  act(() => result.current.set(3))
  act(() => result.current.undo())
  expect(result.current.state).toBe(0)
})

test('new_edit_clears_redo_and_noop_is_ignored', () => {
  const { result } = renderHook(() => useHistory(0))
  act(() => result.current.set(1)); step()
  act(() => result.current.undo())
  act(() => result.current.set(5))
  expect(result.current.canRedo).toBe(false)
  step()
  act(() => result.current.set((p) => p))
  expect(result.current.canUndo).toBe(true)
  act(() => result.current.undo())
  expect(result.current.state).toBe(0)
})
