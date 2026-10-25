import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from 'react';

import { errorMessage } from '../api/invoke';
import type { Progress } from '../api/types';

/** Whether something is writing to the card, and how far it has got. */
function useSharedState() {
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const begin = useCallback(() => {
    setBusy(true);
    setProgress(null);
  }, []);
  const end = useCallback(() => {
    setBusy(false);
    setProgress(null);
  }, []);
  return { busy, progress, begin, report: setProgress, end };
}

const SharedContext = createContext<ReturnType<typeof useSharedState> | null>(null);

/** One busy flag and progress for the whole app: only one thing writes to the card at a time. */
export function OperationProvider({ children }: { children: ReactNode }) {
  return createElement(SharedContext.Provider, { value: useSharedState() }, children);
}

/**
 * Runs one async task at a time, tracking its failure. Busy and progress are
 * shared with every other user of this hook under an `OperationProvider`.
 * `run` resolves to `undefined` when the task failed (the message is in `error`).
 */
export function useOperation() {
  const local = useSharedState();
  const shared = useContext(SharedContext) ?? local;
  const { begin, report, end } = shared;
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(
    async <T>(task: (onProgress: (progress: Progress) => void) => Promise<T>) => {
      begin();
      setError(null);
      try {
        return await task(report);
      } catch (failure) {
        setError(errorMessage(failure));
        return undefined;
      } finally {
        end();
      }
    },
    [begin, report, end],
  );

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return { busy: shared.busy, progress: shared.progress, error, run, clearError };
}
