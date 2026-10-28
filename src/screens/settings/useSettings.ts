import { useCallback, useEffect, useRef, useState } from 'react';

import {
  applySettings,
  connectSettings,
  disconnectSettings,
  rescanPlayerLibrary,
  settingsStatus,
  settingsValues,
  type Refusal,
  type SettingsSchema,
  type SettingsSnapshot,
  type SettingValue,
} from '../../api/settings';
import { errorCode, errorMessage } from '../../api/invoke';
import { t } from '../../i18n';
import { merged, refusalMessage, stage, type Values } from './logic';

const POLL_MS = 4000;

/** What stops the settings from opening, so the panel can say what to do. */
export type Problem =
  'unreachable' | 'unsupported' | 'pin' | 'wrong_pin' | 'busy' | 'rate_limited' | null;

/** How often a busy player is asked again on its own, and how soon. */
const BUSY_TRIES = 5;
const BUSY_RETRY_MS = 2000;

export type Phase =
  | { kind: 'loading' }
  | { kind: 'connect'; problem: Problem; error: string | null }
  | { kind: 'ready' };

/** What happened to a staged change after Apply: `applied`, or when it will take effect. */
export type Outcome = 'applied' | 'next_track' | 'next_use' | 'next_boot';

interface Data {
  schema: SettingsSchema;
  values: Values;
  revision: number;
  epoch: number;
  writable: boolean;
}

const fromSnapshot = (schema: SettingsSchema, snap: SettingsSnapshot): Data => ({
  schema,
  values: snap.values,
  revision: snap.revision,
  epoch: snap.epoch,
  writable: snap.writable,
});

/**
 * The player's settings over ADB: what is on the player, what the user has staged, and the
 * conversation with the player (revision polling, applying, reloading after a conflict).
 * The PIN is passed to `connect` once and kept by the app's backend, never here.
 */
export function usePlayerSettings(serial: string) {
  const [phase, setPhase] = useState<Phase>({ kind: 'loading' });
  const [data, setData] = useState<Data | null>(null);
  const [staged, setStaged] = useState<Values>({});
  const [outcomes, setOutcomes] = useState<Record<string, Outcome>>({});
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [rescanState, setRescanState] = useState<'idle' | 'started'>('idle');
  const latest = useRef<Data | null>(null);
  const working = useRef(false);
  /** The backend session this view talks to; every call after connecting quotes it. */
  const session = useRef<number | null>(null);
  const alive = useRef(true);
  const busyTries = useRef(0);
  const reopen = useRef<(() => Promise<void>) | null>(null);
  const retryTimer = useRef<number | undefined>(undefined);

  const isWorking = () => working.current;
  const setCurrent = useCallback((next: Data | null) => {
    latest.current = next;
    setData(next);
  }, []);

  const open = useCallback(
    async (pin?: string): Promise<void> => {
      window.clearTimeout(retryTimer.current); // a manual Retry replaces a pending automatic one
      setPhase({ kind: 'loading' });
      setError(null);
      setStaged({});
      setOutcomes({});
      try {
        const result = await connectSettings(serial, pin);
        session.current = result.session;
        if (!alive.current) {
          // The screen was left while connecting: nobody wants this session.
          void disconnectSettings(result.session).catch(() => undefined);
          return;
        }
        if (result.state === 'busy' && busyTries.current < BUSY_TRIES) {
          busyTries.current += 1;
          setCurrent(null);
          setPhase({ kind: 'connect', problem: 'busy', error: null });
          retryTimer.current = window.setTimeout(() => void reopen.current?.(), BUSY_RETRY_MS);
          return;
        }
        busyTries.current = 0;
        if (result.state === 'ready') {
          setCurrent(fromSnapshot(result.schema, result.snapshot));
          setPhase({ kind: 'ready' });
        } else {
          setCurrent(null);
          const problem: Problem =
            result.state === 'pin_required' ? (result.wrong ? 'wrong_pin' : 'pin') : result.state;
          setPhase({ kind: 'connect', problem, error: null });
        }
      } catch (failure) {
        setCurrent(null);
        // A dropped connection after the forward worked means the player stopped answering.
        const problem: Problem = errorCode(failure) === 'network' ? 'unreachable' : null;
        setPhase({
          kind: 'connect',
          problem,
          error: problem ? null : errorMessage(failure),
        });
      }
    },
    [serial, setCurrent],
  );

  useEffect(() => {
    reopen.current = () => open();
  }, [open]);

  // Open on arrival and close (forward removed, PIN forgotten) on leaving.
  useEffect(() => {
    alive.current = true;
    void open();
    return () => {
      alive.current = false;
      window.clearTimeout(retryTimer.current);
      const id = session.current;
      session.current = null;
      if (id !== null) void disconnectSettings(id).catch(() => undefined);
    };
  }, [open]);

  /** Back to the connect panel because the player stopped answering or the PIN changed. */
  const lost = useCallback(
    (problem: Problem) => {
      setCurrent(null);
      setStaged({});
      setPhase({ kind: 'connect', problem, error: null });
    },
    [setCurrent],
  );

  /** Reads everything again, dropping what was cached. */
  const reload = useCallback(
    async (message?: string) => {
      const current = latest.current;
      if (!current) return;
      const id = session.current;
      if (id === null) return;
      const reply = await settingsValues(id);
      if (reply.result === 'refused') {
        if (reply.refusal.status === 401) lost('wrong_pin');
        else setError(refusalMessage(reply.refusal, current.schema));
        return;
      }
      const next = fromSnapshot(current.schema, reply.data);
      setCurrent(next);
      // A staged value the player now has is nothing to send any more.
      setStaged((old) =>
        Object.fromEntries(
          Object.entries(old).filter(([key, value]) => next.values[key] !== value),
        ),
      );
      if (message) setNotice(message);
    },
    [lost, setCurrent],
  );

  /** One poll: a restart reloads everything, a newer revision only what changed. */
  const poll = useCallback(async () => {
    const current = latest.current;
    if (!current || working.current) return;
    try {
      const id = session.current;
      if (id === null) return;
      const status = await settingsStatus(id);
      if (status.result === 'refused') {
        if (status.refusal.status === 401) lost('wrong_pin');
        return;
      }
      const { settingsEpoch: epoch, settingsRevision: revision } = status.data;
      if (epoch === null || revision === null) return;
      if (epoch !== current.epoch) {
        setStaged({});
        await reload(
          t(
            'The player was restarted. Its settings were reloaded and your unsent changes dropped.',
          ),
        );
        return;
      }
      if (revision === current.revision) return;
      const reply = await settingsValues(id, { revision: current.revision, epoch: current.epoch });
      if (reply.result === 'refused') return;
      const snap = reply.data;
      if (isWorking()) return; // an Apply began while this was in flight
      const values = snap.full ? snap.values : { ...current.values, ...snap.values };
      setCurrent({
        ...current,
        values,
        revision: snap.revision,
        epoch: snap.epoch,
        writable: snap.writable,
      });
      // Something changed on the device: unsent edits of those same settings give way to it.
      const changed = Object.keys(snap.values).filter(
        (key) => current.values[key] !== snap.values[key],
      );
      setStaged((old) => {
        const dropped = Object.keys(old).filter((key) => changed.includes(key));
        if (dropped.length > 0)
          setNotice(t('A setting was changed on the player; your unsent edit of it was dropped.'));
        return Object.fromEntries(Object.entries(old).filter(([key]) => !dropped.includes(key)));
      });
    } catch (failure) {
      // A session that was replaced is no longer ours to poll; a dropped link is worth saying.
      if (errorCode(failure) === 'network') lost('unreachable');
    }
  }, [lost, reload, setCurrent]);

  const ready = phase.kind === 'ready';
  useEffect(() => {
    if (!ready) return undefined;
    const id = window.setInterval(() => void poll(), POLL_MS);
    return () => {
      window.clearInterval(id);
    };
  }, [ready, poll]);

  const change = useCallback((key: string, value: SettingValue) => {
    const current = latest.current;
    if (!current) return;
    setOutcomes((old) => Object.fromEntries(Object.entries(old).filter(([k]) => k !== key)));
    setStaged((old) => stage(current.schema, current.values, old, key, value));
  }, []);

  const discard = useCallback(() => {
    setStaged({});
  }, []);

  /** A refusal: say why, and reread the player when what we know may be out of date. */
  const refused = useCallback(
    async (refusal: Refusal, schema: SettingsSchema) => {
      if (refusal.status === 401) {
        lost('wrong_pin');
        return;
      }
      setError(refusalMessage(refusal, schema));
      // A conflict or a timed-out change: the player's values are the truth now.
      if (refusal.code === 'revision_mismatch' || refusal.code === 'player_busy') await reload();
    },
    [lost, reload],
  );

  /** Sends the staged changes. `acknowledge` lists the loud-volume keys the user confirmed. */
  const apply = useCallback(
    async (acknowledge: string[]) => {
      const current = latest.current;
      if (!current || working.current || Object.keys(staged).length === 0) return;
      const id = session.current;
      if (id === null) return;
      const sent = staged;
      working.current = true;
      setBusy(true);
      setError(null);
      setNotice(null);
      try {
        const reply = await applySettings(id, {
          epoch: current.epoch,
          baseRevision: current.revision,
          changes: sent,
          acknowledge,
        });
        if (reply.result === 'ok') {
          const result: Record<string, Outcome> = {};
          for (const key of reply.data.applied) result[key] = 'applied';
          for (const p of reply.data.pending) result[p.key] = p.reason as Outcome;
          setOutcomes(result);
          // Only what was sent is done; edits made while it was in flight stay staged.
          setStaged((old) =>
            Object.fromEntries(Object.entries(old).filter(([key, value]) => sent[key] !== value)),
          );
          await reload();
        } else {
          await refused(reply.refusal, current.schema);
        }
      } catch (failure) {
        if (errorCode(failure) === 'network') lost('unreachable');
        else setError(errorMessage(failure));
      } finally {
        working.current = false;
        setBusy(false);
      }
    },
    [staged, reload, lost, refused],
  );

  const rescan = useCallback(async () => {
    const current = latest.current;
    const id = session.current;
    if (!current || id === null) return;
    setError(null);
    try {
      const reply = await rescanPlayerLibrary(id);
      if (reply.result === 'ok') setRescanState('started');
      else setError(refusalMessage(reply.refusal, current.schema));
    } catch (failure) {
      if (errorCode(failure) === 'network') lost('unreachable');
      else setError(errorMessage(failure));
    }
  }, [lost]);

  const effective = data ? merged(data.values, staged) : {};
  return {
    phase,
    schema: data?.schema ?? null,
    values: data?.values ?? {},
    effective,
    revision: data?.revision ?? null,
    writable: data?.writable ?? false,
    staged,
    outcomes,
    error,
    notice,
    busy,
    rescanState,
    connect: open,
    reload: () => reload(),
    change,
    discard,
    apply,
    rescan,
    dismissError: () => {
      setError(null);
    },
    dismissNotice: () => {
      setNotice(null);
    },
  };
}
