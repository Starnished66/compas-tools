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

import { identifyDevice, listDevicesQuick } from '../api/adb';
import { cardForgetIfMissing, cardPresent, listCards } from '../api/card';
import { errorCode, errorMessage } from '../api/invoke';
import type { CardInfo, DeviceInfo, DeviceView } from '../api/types';
import { useCard } from './CardContext';
import { usePolling } from './usePolling';

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
  /** Why the last look for ADB players failed; null when it worked. */
  adbError: string | null;
  /** The selected drive is gone (unplugged, or the mode was switched). */
  cardMissing: boolean;
  /** Looks again for drives and ADB players. */
  rescan: () => Promise<void>;
  /** The ADB screen has just listed devices itself; keep the mode in step. */
  reportDevices: (devices: DeviceView[]) => void;
}

const idle: ModeValue = {
  cards: [],
  adbPlayers: [],
  scanning: false,
  adbError: null,
  cardMissing: false,
  rescan: () => Promise.resolve(),
  reportDevices: () => undefined,
};

const ModeContext = createContext<ModeValue>(idle);

/** How often the window looks while everything is calm. */
const POLL_MS = 2500;
/** How soon it looks again while something is changing (a device coming up, a drive gone). */
const RETRY_MS = 1000;
const MAX_BACKOFF_MS = 5000;
/** A drive is dropped only when this many looks in a row found it gone. */
const MISSES_TO_DROP = 2;

/** A device adb lists as `device` and that looks like the player (others are never touched). */
const isPlayer = (d: DeviceView) => d.state === 'device' && d.likely_player;

/** Listed, but adb has not finished with it: waiting for the user to accept, or still booting. */
const isSettling = (d: DeviceView) => d.state !== 'device';

/** Shown the moment adb reports the player, until it answers who it is. */
const unidentified = (serial: string): DeviceInfo => ({
  serial,
  board: null,
  firmware: 'unknown',
  player_version: null,
  plugin_api: null,
  sd_mounted: false,
});

interface Listing {
  views: DeviceView[];
  error: string | null;
}

async function listAdb(): Promise<Listing> {
  try {
    return { views: await listDevicesQuick(), error: null };
  } catch (failure) {
    // No adb installed yet is not a failure; the Install screen offers the download.
    if (errorCode(failure) === 'no_adb') return { views: [], error: null };
    return { views: [], error: errorMessage(failure) };
  }
}

export function ModeProvider({ children }: { children: ReactNode }) {
  const { card, select, forget } = useCard();
  const [cards, setCards] = useState<CardInfo[]>([]);
  const [adbPlayers, setAdbPlayers] = useState<DeviceInfo[]>([]);
  const [adbError, setAdbError] = useState<string | null>(null);
  const [cardMissing, setCardMissing] = useState(false);
  const [scanning, setScanning] = useState(false);
  const selected = useRef(card);
  useEffect(() => {
    selected.current = card;
  }, [card]);

  // Only the newest pass may write results; a pass also stops writing once the provider is gone.
  const latest = useRef(0);
  const alive = useRef(true);
  const inFlight = useRef<Promise<number> | null>(null);
  const manual = useRef(0);
  const failures = useRef(0);
  const misses = useRef<{ root: string; count: number } | null>(null);
  // Set by a look the user (or a screen change) asked for: only those pick the only drive.
  const pickDrive = useRef(false);
  // What each connected player said it is, until it disconnects, and who is being asked.
  const known = useRef(new Map<string, DeviceInfo>());
  const asking = useRef(new Set<string>());

  const playersOf = useCallback((views: DeviceView[]): DeviceInfo[] => {
    const now = new Set(views.filter(isPlayer).map((v) => v.serial));
    for (const serial of [...known.current.keys()])
      if (!now.has(serial)) known.current.delete(serial);
    return views.filter(isPlayer).map((v) => {
      // The backend forgets a player at an install or when it is replugged; so does this.
      if (v.info) known.current.set(v.serial, v.info);
      else known.current.delete(v.serial);
      return known.current.get(v.serial) ?? unidentified(v.serial);
    });
  }, []);

  /** Asks the players that have not answered yet who they are; the list fills in as they do. */
  const identify = useCallback((views: DeviceView[]) => {
    for (const { serial } of views.filter(isPlayer)) {
      if (known.current.has(serial) || asking.current.has(serial)) continue;
      asking.current.add(serial);
      identifyDevice(serial)
        .then((info) => {
          // Not for a player that left meanwhile, nor after the provider is gone.
          if (!alive.current || !asking.current.has(serial)) return;
          known.current.set(serial, info);
          setAdbPlayers((now) => now.map((p) => (p.serial === serial ? info : p)));
        })
        .catch(() => undefined) // asked again on the next look
        .finally(() => {
          asking.current.delete(serial);
        });
    }
  }, []);

  /** One look at drives and adb. Resolves with how long to wait before the next one. */
  const look = useCallback(async (): Promise<number> => {
    const pass = ++latest.current;
    const wantPick = pickDrive.current;
    pickDrive.current = false;
    const [found, listing] = await Promise.all([listCards().catch(() => []), listAdb()]);
    if (pass !== latest.current) return POLL_MS;
    setCards(found);
    setAdbError(listing.error);
    if (listing.error) {
      // One failed look says nothing about the players: keep what was known.
      failures.current += 1;
    } else {
      failures.current = 0;
      setAdbPlayers(playersOf(listing.views));
      identify(listing.views);
      for (const serial of [...asking.current])
        if (!listing.views.some((v) => v.serial === serial && isPlayer(v)))
          asking.current.delete(serial);
    }

    let settling = listing.views.some(isSettling);
    const current = selected.current;
    if (current && !found.some((c) => c.root === current.root)) {
      // Not listed: gone, or a folder the user picked by hand. Only its folder can tell.
      const present = await cardPresent().catch(() => true);
      if (pass !== latest.current) return POLL_MS;
      if (present) {
        misses.current = null;
        setCardMissing(false);
      } else {
        const count = misses.current?.root === current.root ? misses.current.count + 1 : 1;
        misses.current = { root: current.root, count };
        settling = true;
        if (count >= MISSES_TO_DROP) {
          setCardMissing(true);
          // The backend drops it only if it is still this card and still cannot be opened.
          const dropped = await cardForgetIfMissing(current.root).catch(() => false);
          if (pass !== latest.current) return POLL_MS;
          // The user may have chosen another player while this was asked.
          if (dropped && selected.current?.root === current.root) {
            selected.current = null;
            forget(current.root);
          }
          if (!dropped) setCardMissing(false);
        }
      }
    } else {
      misses.current = null;
      setCardMissing(false);
      // One player on the computer and none chosen yet: that is the one.
      const [only] = found;
      if (wantPick && !current && found.length === 1 && only)
        await select(only.root).catch(() => undefined);
    }

    if (listing.error) return Math.min(RETRY_MS * 2 ** failures.current, MAX_BACKOFF_MS);
    return settling ? RETRY_MS : POLL_MS;
  }, [forget, identify, playersOf, select]);

  /** Looks now, or joins the look already under way: two never run at once. */
  const scan = useCallback(() => {
    inFlight.current ??= look().finally(() => {
      inFlight.current = null;
    });
    return inFlight.current;
  }, [look]);

  const rescan = useCallback(async () => {
    pickDrive.current = true;
    manual.current += 1;
    setScanning(true);
    try {
      await scan();
    } finally {
      manual.current -= 1;
      if (manual.current === 0) setScanning(false);
    }
  }, [scan]);

  const reportDevices = useCallback(
    (devices: DeviceView[]) => {
      setAdbPlayers(playersOf(devices));
    },
    [playersOf],
  );

  useEffect(() => {
    alive.current = true;
    void rescan();
    return () => {
      alive.current = false;
      latest.current += 1; // drops the answers still on their way
    };
  }, [rescan]);

  // Cables are moved and modes switched while a screen stays open.
  usePolling(scan, POLL_MS);

  const value = useMemo(
    () => ({ cards, adbPlayers, scanning, adbError, cardMissing, rescan, reportDevices }),
    [cards, adbPlayers, scanning, adbError, cardMissing, rescan, reportDevices],
  );
  return <ModeContext.Provider value={value}>{children}</ModeContext.Provider>;
}

export function useMode(): ModeValue {
  return useContext(ModeContext);
}
