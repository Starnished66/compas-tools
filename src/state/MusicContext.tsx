import { createContext, useContext, type ReactNode } from 'react';

import { useMusicTransfer } from '../hooks/useMusicTransfer';
import { useCard } from './CardContext';

const MusicContext = createContext<ReturnType<typeof useMusicTransfer> | null>(null);

/** Keeps the Music screen's sources, folder and options while other screens are open. */
export function MusicProvider({ children }: { children: ReactNode }) {
  const { card } = useCard();
  return (
    <MusicContext.Provider value={useMusicTransfer(card?.root ?? null)}>
      {children}
    </MusicContext.Provider>
  );
}

export function useMusic() {
  const value = useContext(MusicContext);
  if (!value) throw new Error('useMusic must be used inside MusicProvider');
  return value;
}
