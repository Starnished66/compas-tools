import { useCallback, useState } from 'react';

import {
  installPlugin,
  moveRecordAside,
  setPluginEnabled,
  uninstallPlugin,
  updateAllPlugins,
  updatePlugin,
} from '../api/catalog';
import type { PluginEntry, Progress, UpdateAllReport } from '../api/types';
import { useOperation } from './useOperation';

type ChangeKind = 'install' | 'update';

export type Dialog =
  | { type: 'replace'; id: string; kind: ChangeKind; files: string[] }
  | { type: 'replace_all'; items: { id: string; files: string[] }[] }
  | { type: 'remove'; id: string }
  | null;

/** Every way the user changes the card from a catalog view. */
export type PluginActions = ReturnType<typeof usePluginActions>;

export function usePluginActions(reload: () => Promise<void>) {
  const operation = useOperation();
  const { run } = operation;
  const [activeId, setActiveId] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [report, setReport] = useState<UpdateAllReport | null>(null);

  /** Runs `task`, then re-reads the card, whatever the result. */
  const track = useCallback(
    async <T>(
      id: string | null,
      task: (onProgress: (progress: Progress) => void) => Promise<T>,
    ) => {
      setActiveId(id);
      const result = await run(task);
      setActiveId(null);
      await reload();
      return result;
    },
    [run, reload],
  );

  const change = useCallback(
    async (id: string, kind: ChangeKind, force: boolean) => {
      const apply = kind === 'install' ? installPlugin : updatePlugin;
      const outcome = await track(id, (onProgress) => apply(id, force, onProgress));
      if (outcome && outcome !== 'done') {
        setDialog({ type: 'replace', id, kind, files: outcome.needs_confirmation });
      }
    },
    [track],
  );

  const confirm = useCallback(async () => {
    const current = dialog;
    setDialog(null);
    if (current?.type === 'replace') await change(current.id, current.kind, true);
    if (current?.type === 'replace_all') {
      const ids = current.items.map((item) => item.id);
      const result = await track(null, (onProgress) => updateAllPlugins(ids, onProgress));
      if (result) setReport(result);
    }
    if (current?.type === 'remove') {
      await track(current.id, (onProgress) => uninstallPlugin(current.id, onProgress));
    }
  }, [dialog, change, track]);

  return {
    busy: operation.busy,
    progress: operation.progress,
    error: operation.error,
    clearError: operation.clearError,
    activeId,
    dialog,
    report,
    dismissReport: useCallback(() => {
      setReport(null);
    }, []),
    cancelDialog: useCallback(() => {
      setDialog(null);
    }, []),
    confirm,
    /** Drops the pending dialog and report; they belong to the card they came from. */
    reset: useCallback(() => {
      setDialog(null);
      setReport(null);
    }, []),
    install: useCallback((id: string) => change(id, 'install', false), [change]),
    update: useCallback((id: string) => change(id, 'update', false), [change]),
    moveRecordAside: useCallback(() => track(null, () => moveRecordAside()), [track]),
    askUninstall: useCallback((id: string) => {
      setDialog({ type: 'remove', id });
    }, []),
    toggle: useCallback(
      (entry: PluginEntry, enabled: boolean) =>
        track(null, async () => {
          if (entry.script) await setPluginEnabled(entry.script, enabled);
        }),
      [track],
    ),
    updateAll: useCallback(async () => {
      setReport(null);
      const result = await track(null, (onProgress) => updateAllPlugins([], onProgress));
      if (result) setReport(result);
      // One question for everything that would replace local files.
      if (result && result.needs_confirmation.length > 0) {
        setDialog({ type: 'replace_all', items: result.needs_confirmation });
      }
    }, [track]),
  };
}
