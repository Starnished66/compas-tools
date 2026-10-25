import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';

import { usePluginActions, type PluginActions } from '../hooks/usePluginActions';
import { errorMessage } from '../api/invoke';
import { loadEntries } from '../api/catalog';
import type { PluginEntry } from '../api/types';
import { useCard } from './CardContext';

interface CatalogContextValue {
  entries: PluginEntry[];
  loading: boolean;
  error: string | null;
  /** Re-evaluates the catalog against the card; `refresh` fetches the index again. */
  load: (refresh: boolean) => Promise<void>;
  /** Install, update and remove state lives here so it survives switching views. */
  actions: PluginActions;
}

const CatalogContext = createContext<CatalogContextValue | null>(null);

/** One shared catalog for the Plugins, Layouts and Themes views. */
export function CatalogProvider({ children }: { children: ReactNode }) {
  const { card } = useCard();
  const root = card?.root ?? null;
  const [entries, setEntries] = useState<PluginEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const latest = useRef(0);

  const load = useCallback(
    async (refresh: boolean) => {
      if (!root) return;
      // Only the newest request may write results; the card may have changed.
      const request = ++latest.current;
      setLoading(true);
      setError(null);
      try {
        const loaded = await loadEntries(refresh);
        if (request === latest.current) setEntries(loaded);
      } catch (failure) {
        if (request === latest.current) setError(errorMessage(failure));
      } finally {
        if (request === latest.current) setLoading(false);
      }
    },
    [root],
  );
  const actions = usePluginActions(() => load(false));

  useEffect(() => {
    latest.current += 1; // drop answers still pending for the previous card
    setEntries([]);
    void load(true);
  }, [load]);

  return (
    <CatalogContext.Provider value={{ entries, loading, error, load, actions }}>
      {children}
    </CatalogContext.Provider>
  );
}

export function useCatalog(): CatalogContextValue {
  const value = useContext(CatalogContext);
  if (!value) throw new Error('useCatalog must be used inside CatalogProvider');
  return value;
}
