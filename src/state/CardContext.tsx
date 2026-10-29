import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

import { clearCard, currentCard, selectCard } from '../api/card';
import type { CardInfo, Model } from '../api/types';

interface CardContextValue {
  card: CardInfo | null;
  /** The model from `device.json`, or the one the user picked when it is missing. */
  model: Model | null;
  chooseModel: (model: Model) => void;
  select: (path: string) => Promise<void>;
  refresh: () => Promise<void>;
  clear: () => Promise<void>;
  /** The backend has already deselected the player; drop it here too. */
  forget: (root: string) => void;
}

const CardContext = createContext<CardContextValue | null>(null);

export function CardProvider({ children }: { children: ReactNode }) {
  const [card, setCard] = useState<CardInfo | null>(null);
  const [chosen, setChosen] = useState<{ root: string; model: Model } | null>(null);

  const select = useCallback(async (path: string) => {
    setCard(await selectCard(path));
  }, []);
  const refresh = useCallback(async () => {
    setCard(await currentCard());
  }, []);
  const clear = useCallback(async () => {
    await clearCard();
    setCard(null);
  }, []);
  const forget = useCallback((root: string) => {
    // A card chosen meanwhile stays.
    setCard((now) => (now?.root === root ? null : now));
  }, []);
  const chooseModel = useCallback(
    (model: Model) => {
      if (card) setChosen({ root: card.root, model });
    },
    [card],
  );

  const model = card?.device?.model ?? (chosen && chosen.root === card?.root ? chosen.model : null);
  const value = useMemo(
    () => ({ card, model, chooseModel, select, refresh, clear, forget }),
    [card, model, chooseModel, select, refresh, clear, forget],
  );
  return <CardContext.Provider value={value}>{children}</CardContext.Provider>;
}

export function useCard(): CardContextValue {
  const value = useContext(CardContext);
  if (!value) throw new Error('useCard must be used inside CardProvider');
  return value;
}
