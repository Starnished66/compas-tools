import { useCallback, useEffect, useRef, useState } from 'react';

import * as api from '../../api/playerPlugins';
import type { Reply } from '../../api/settings';
import { t } from '../../i18n';

const RUNNING = new Set(['refreshing', 'installing', 'updating', 'updating_all', 'uninstalling']);
class PlayerPluginError extends Error {}

function unwrap<T>(reply: Reply<T>): T {
  if (reply.result === 'ok') return reply.data;
  const { status, code } = reply.refusal;
  if (status === 401 || status === 403)
    throw new PlayerPluginError(t('Reconnect with the current player PIN.'));
  if (code === 'catalog_required')
    throw new PlayerPluginError(t('Refresh the store catalog first.'));
  if (status === 409)
    throw new PlayerPluginError(t('The player is busy. Check its screen and try again.'));
  throw new PlayerPluginError(
    t('The request could not be completed. Check the player status before trying again.'),
  );
}

/** Complete bounded pages. Never turn a partial inventory into a successful full list. */
async function pages<
  T extends { id: string },
  P extends { offset: number; limit: number; total: number },
>(
  fetch: (offset: number) => Promise<Reply<P>>,
  rows: (page: P) => T[],
  maximum: number,
): Promise<{ first: P; items: T[] }> {
  let offset = 0;
  let first: P | undefined;
  const items: T[] = [];
  const ids = new Set<string>();
  do {
    const page = unwrap(await fetch(offset));
    first ??= page;
    const batch = rows(page);
    if (
      page.offset !== offset ||
      page.limit !== 200 ||
      !Number.isSafeInteger(page.total) ||
      page.total < 0 ||
      page.total > maximum ||
      page.total !== first.total ||
      batch.length > 200 ||
      offset + batch.length > page.total
    ) {
      throw new PlayerPluginError(
        t('The player returned an incomplete list. Reload to try again.'),
      );
    }
    if (!batch.length && offset < page.total)
      throw new PlayerPluginError(
        t('The player returned an incomplete list. Reload to try again.'),
      );
    for (const item of batch) {
      if (ids.has(item.id))
        throw new PlayerPluginError(
          t('The player returned an incomplete list. Reload to try again.'),
        );
      ids.add(item.id);
    }
    items.push(...batch);
    offset += batch.length;
    if (items.length > maximum)
      throw new PlayerPluginError(
        t('The player returned an incomplete list. Reload to try again.'),
      );
    if (offset >= page.total) break;
  } while (offset < maximum);
  return { first, items };
}

export function usePlayerPlugins(session: number, features: string[]) {
  const [store, setStore] = useState<api.StoreSnapshot | null>(null);
  const [installed, setInstalled] = useState<api.InstalledSnapshot | null>(null);
  const [layouts, setLayouts] = useState<api.LayoutSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [statusUnknown, setStatusUnknown] = useState(false);
  const unknown = useRef(false);
  const generation = useRef(0);
  const working = useRef(false);
  const refreshNeeded = useRef(false);
  const supported = features.join(',');

  const read = useCallback(
    async (token: number) => {
      if (token !== generation.current) return;
      refreshNeeded.current = false;
      const has = (feature: string) => supported.split(',').includes(feature);
      if (has('plugin_store')) {
        const result = await pages(
          (offset) => api.pluginStore(session, offset, 200),
          (page) => page.plugins,
          600,
        );
        if (token !== generation.current) return;
        setStore({ ...result.first, plugins: result.items });
        refreshNeeded.current = RUNNING.has(result.first.state);
      }
      if (has('plugin_management')) {
        const result = await pages(
          (offset) => api.installedPlugins(session, offset, 200),
          (page) => page.plugins,
          400,
        );
        if (token !== generation.current) return;
        setInstalled({ ...result.first, plugins: result.items });
        refreshNeeded.current ||= result.first.refresh_pending;
      }
      if (has('player_layouts')) {
        const result = await pages(
          (offset) => api.playerLayouts(session, offset, 200),
          (page) => page.layouts,
          1024,
        );
        if (token !== generation.current) return;
        setLayouts({ ...result.first, layouts: result.items });
      }
    },
    [session, supported],
  );

  const reload = useCallback(
    async (manual = true) => {
      if (working.current) return;
      working.current = true;
      const token = generation.current;
      setBusy(true);
      if (manual) setError(null);
      try {
        refreshNeeded.current = false;
        await read(token);
        if (manual && token === generation.current) {
          unknown.current = false;
          setStatusUnknown(false);
        }
      } catch (failure) {
        if (token === generation.current) {
          refreshNeeded.current = false;
          setError(
            failure instanceof PlayerPluginError
              ? failure.message
              : t('Could not read the player.'),
          );
        }
      } finally {
        if (token === generation.current) {
          working.current = false;
          setBusy(false);
        }
      }
    },
    [read],
  );

  useEffect(() => {
    const token = generation.current + 1;
    generation.current = token;
    working.current = false;
    unknown.current = false;
    setStatusUnknown(false);
    void reload();
    const timer = window.setInterval(() => {
      if (refreshNeeded.current) void reload(false);
    }, 1500);
    return () => {
      generation.current = token + 1;
      window.clearInterval(timer);
    };
  }, [reload]);

  const execute = async (action: () => Promise<Reply<api.ActionResult>>) => {
    if (working.current || unknown.current) return;
    working.current = true;
    const token = generation.current;
    setBusy(true);
    setError(null);
    try {
      const accepted = unwrap(await action());
      if (!accepted.accepted)
        throw new PlayerPluginError(t('The player did not accept the operation.'));
      if (token !== generation.current) return;
      refreshNeeded.current = true;
      await read(token);
    } catch (failure) {
      if (token === generation.current) {
        unknown.current = true;
        setStatusUnknown(true);
        refreshNeeded.current = false;
        setError(
          failure instanceof PlayerPluginError
            ? failure.message
            : t('The request could not be completed. Check the player status before trying again.'),
        );
      }
    } finally {
      if (token === generation.current) {
        working.current = false;
        setBusy(false);
      }
    }
  };

  return {
    store,
    installed,
    layouts,
    busy,
    error,
    statusUnknown,
    reload,
    execute,
    changing: (!!store && RUNNING.has(store.state)) || !!installed?.refresh_pending,
  };
}
