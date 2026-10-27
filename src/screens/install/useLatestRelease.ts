import { useEffect, useState } from 'react';

import { errorMessage } from '../../api/invoke';
import { latestRelease } from '../../api/releases';
import type { LatestRelease } from '../../api/types';

/**
 * The latest player release, compared with `installedVersion` (read over adb). `null` and an
 * empty version both mean the version is not known; it is read again when the version changes.
 */
export function useLatestRelease(installedVersion: string | null) {
  const [latest, setLatest] = useState<LatestRelease | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const live = { current: true };
    setLatest(null);
    setError(null);
    void (async () => {
      try {
        const found = await latestRelease(attempt > 0, installedVersion ?? '');
        if (live.current) setLatest(found);
      } catch (failure) {
        if (live.current) setError(errorMessage(failure));
      }
    })();
    return () => {
      live.current = false;
    };
  }, [installedVersion, attempt]);

  return {
    latest,
    error,
    /** Asks GitHub again instead of using the short-lived copy. */
    reload: () => {
      setAttempt((n) => n + 1);
    },
  };
}
