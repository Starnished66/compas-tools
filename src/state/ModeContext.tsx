import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';

import { adbStatus, listDevices } from '../api/adb';
import { listCards } from '../api/card';
import type { CardInfo, DeviceInfo, DeviceView } from '../api/types';
import { useCard } from './CardContext';

/**
 * How the player is reachable right now: its files as a drive (Storage mode), over USB with ADB
 * (ADB mode), or both. Screens that need one of them say so when only the other is there.
 */
interface ModeValue {
  /** Players found as drives. */
  cards: CardInfo[];
  /** Players seen over ADB, with what they reported. */
  adbPlayers: DeviceInfo[];
  scanning: boolean;
  /** Looks again for drives and ADB players. */
  rescan: () => Promise<void>;
  /** The ADB screen has just listed devices itself; keep the mode in step. */
  reportDevices: (devices: DeviceView[]) => void;
}

const idle: ModeValue = {
  cards: [],
  adbPlayers: [],
  scanning: false,
  rescan: () => Promise.resolve(),
  reportDevices: () => undefined,
};

const ModeContext = createContext<ModeValue>(idle);

const playersIn = (devices: DeviceView[]): DeviceInfo[] =>
  devices.flatMap((d) => (d.state === 'device' && d.info ? [d.info] : []));

export function ModeProvider({ children }: { children: ReactNode }) {
  const { card, select } = useCard();
  const [cards, setCards] = useState<CardInfo[]>([]);
  const [adbPlayers, setAdbPlayers] = useState<DeviceInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const selected = useRef(card);
  useEffect(() => {
    selected.current = card;
  }, [card]);

  const rescan = useCallback(async () => {
    setScanning(true);
    try {
      const [found, devices] = await Promise.all([
        listCards().catch(() => []),
        adbStatus()
          .then((status) => (status.found ? listDevices() : []))
          .catch(() => []),
      ]);
      setCards(found);
      setAdbPlayers(playersIn(devices));
      // One player on the computer and none chosen yet: that is the one.
      const [only] = found;
      if (!selected.current && found.length === 1 && only)
        await select(only.root).catch(() => undefined);
    } finally {
      setScanning(false);
    }
  }, [select]);

  const reportDevices = useCallback((devices: DeviceView[]) => {
    setAdbPlayers(playersIn(devices));
  }, []);

  useEffect(() => {
    void rescan();
  }, [rescan]);

  const value = useMemo(
    () => ({ cards, adbPlayers, scanning, rescan, reportDevices }),
    [cards, adbPlayers, scanning, rescan, reportDevices],
  );
  return <ModeContext.Provider value={value}>{children}</ModeContext.Provider>;
}

export function useMode(): ModeValue {
  return useContext(ModeContext);
}
