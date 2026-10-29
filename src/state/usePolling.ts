import { useEffect, useRef, useState } from 'react';

/** Whether the window is visible and has the focus: nobody is looking at it otherwise. */
export function useWindowActive(): boolean {
  const read = () => document.visibilityState === 'visible' && document.hasFocus();
  const [active, setActive] = useState(read);
  useEffect(() => {
    const update = () => {
      setActive(read());
    };
    window.addEventListener('focus', update);
    window.addEventListener('blur', update);
    document.addEventListener('visibilitychange', update);
    update();
    return () => {
      window.removeEventListener('focus', update);
      window.removeEventListener('blur', update);
      document.removeEventListener('visibilitychange', update);
    };
  }, []);
  return active;
}

/**
 * Runs `task` now and then again after each run finishes, only while the window is active.
 * The task may answer with the number of milliseconds to wait before the next run (a short
 * retry while something is still settling); otherwise `intervalMs` is used. Runs never overlap,
 * and coming back to the window runs the task straight away.
 */
export function usePolling(task: () => Promise<number | undefined>, intervalMs: number): void {
  const latest = useRef(task);
  useEffect(() => {
    latest.current = task;
  }, [task]);
  const active = useWindowActive();

  useEffect(() => {
    if (!active) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      let wait = intervalMs;
      try {
        const hint = await latest.current();
        if (typeof hint === 'number') wait = hint;
      } catch {
        // The task reports its own failures; polling goes on.
      }
      if (!stopped)
        timer = setTimeout(() => {
          void tick();
        }, wait);
    };
    void tick();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [active, intervalMs]);
}
