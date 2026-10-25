import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

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
}

const CatalogContext = createContext<CatalogContextValue | null>(null);

/** One shared catalog for the Plugins, Layouts and Themes views. */
export function CatalogProvider({ children }: { children: ReactNode }) {
  const { card } = useCard();
  const root = card?.root ?? null;
  const [entries, setEntries] = useState<PluginEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(
    async (refresh: boolean) => {
      if (!root) return;
      setLoading(true);
      setError(null);
      try {
        setEntries(await loadEntries(refresh));
      } catch (failure) {
        setError(errorMessage(failure));
      } finally {
        setLoading(false);
      }
    },
    [root],
  );

  useEffect(() => {
    setEntries([]);
    void load(true);
  }, [load]);

  const value = useMemo(() => ({ entries, loading, error, load }), [entries, loading, error, load]);
  return <CatalogContext.Provider value={value}>{children}</CatalogContext.Provider>;
}

export function useCatalog(): CatalogContextValue {
  const value = useContext(CatalogContext);
  if (!value) throw new Error('useCatalog must be used inside CatalogProvider');
  return value;
}
