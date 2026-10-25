import { useCallback, useState } from 'react';

import { errorMessage } from '../api/invoke';
import type { Progress } from '../api/types';

/**
 * Runs one async task at a time, tracking its progress and failure.
 * `run` resolves to `undefined` when the task failed (the message is in `error`).
 */
export function useOperation() {
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(
    async <T>(task: (onProgress: (progress: Progress) => void) => Promise<T>) => {
      setBusy(true);
      setProgress(null);
      setError(null);
      try {
        return await task(setProgress);
      } catch (failure) {
        setError(errorMessage(failure));
        return undefined;
      } finally {
        setBusy(false);
        setProgress(null);
      }
    },
    [],
  );

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return { busy, progress, error, run, clearError };
}
