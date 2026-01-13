import { useState, useCallback } from 'react';

export interface UseFocusModeResult {
  focusMode: boolean;
  enterFocus: () => void;
  exitFocus: () => void;
  toggleFocus: () => void;
}

export function useFocusMode(): UseFocusModeResult {
  const [focusMode, setFocusMode] = useState(false);

  const enterFocus = useCallback(() => setFocusMode(true), []);
  const exitFocus = useCallback(() => setFocusMode(false), []);
  const toggleFocus = useCallback(() => setFocusMode(f => !f), []);

  return { focusMode, enterFocus, exitFocus, toggleFocus };
}
