import { useCallback, useEffect, useRef, useState } from 'react';

import { errorMessage } from '../api/invoke';
import { copyTransfer, planTransfer, updateLibrary } from '../api/music';
import type { CopyView, PlanView } from '../api/types';
import { useOperation } from './useOperation';

/** The sources, destination and options of a transfer, with its plan and result. */
export function useMusicTransfer(enabled: boolean) {
  const [sources, setSources] = useState<string[]>([]);
  const [destDir, setDestDir] = useState('');
  const [mirror, setMirror] = useState(false);
  const [plan, setPlan] = useState<PlanView | null>(null);
  const [planError, setPlanError] = useState<string | null>(null);
  const [result, setResult] = useState<CopyView | null>(null);
  const copying = useOperation();
  const latest = useRef(0);

  const refreshPlan = useCallback(async () => {
    const request = ++latest.current;
    if (!enabled || sources.length === 0) {
      setPlan(null);
      setPlanError(null);
      return;
    }
    try {
      const next = await planTransfer({ sources, destDir, deleteExtra: mirror });
      if (request === latest.current) {
        setPlan(next);
        setPlanError(null);
      }
    } catch (failure) {
      if (request === latest.current) {
        setPlan(null);
        setPlanError(errorMessage(failure));
      }
    }
  }, [enabled, sources, destDir, mirror]);

  useEffect(() => {
    void refreshPlan();
  }, [refreshPlan]);

  const add = useCallback((paths: string[]) => {
    setResult(null);
    setSources((current) => [...new Set([...current, ...paths])]);
  }, []);

  const remove = useCallback((path: string) => {
    setSources((current) => current.filter((p) => p !== path));
  }, []);

  const copy = useCallback(async () => {
    setResult(null);
    const done = await copying.run((onProgress) =>
      copyTransfer({ sources, destDir, deleteExtra: mirror }, onProgress),
    );
    if (done) setResult(done);
    await refreshPlan();
  }, [copying, sources, destDir, mirror, refreshPlan]);

  /** Try again: only the library update runs, never the copy. */
  const retryLibrary = useCallback(async () => {
    const update = await copying.run((onProgress) => updateLibrary(onProgress));
    if (update)
      setResult((current) => current && { ...current, library: { status: 'updated', ...update } });
  }, [copying]);

  return {
    sources,
    destDir,
    mirror,
    plan,
    planError,
    result,
    busy: copying.busy,
    progress: copying.progress,
    error: copying.error,
    clearError: copying.clearError,
    setDestDir,
    setMirror,
    add,
    remove,
    copy,
    retryLibrary,
  };
}
