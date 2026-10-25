import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useEffect, useState } from 'react';

/** Files and folders dropped on the window; `hovering` is true while they are over it. */
export function useFileDrop(onDrop: (paths: string[]) => void): boolean {
  const [hovering, setHovering] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const { type } = event.payload;
        setHovering(type === 'enter' || type === 'over');
        if (event.payload.type === 'drop') onDrop(event.payload.paths);
      })
      .then(
        (stop) => {
          if (active) unlisten = stop;
          else stop();
        },
        () => undefined, // not running inside Tauri
      );
    return () => {
      active = false;
      unlisten?.();
    };
  }, [onDrop]);

  return hovering;
}
