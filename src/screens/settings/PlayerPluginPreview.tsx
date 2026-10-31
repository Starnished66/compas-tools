import { useEffect, useState } from 'react';

import { playerLayoutPreview, storePluginPreview } from '../../api/playerPlugins';
import { t } from '../../i18n';

const cache = new Map<string, string>();
let queue: Promise<unknown> = Promise.resolve();

/** Only one preview request at a time: the player's UI also serves control jobs. */
function enqueue<T>(work: () => Promise<T>): Promise<T> {
  const next = queue.then(work, work);
  queue = next.catch(() => undefined);
  return next;
}

export function PlayerPluginPreview({
  session,
  id,
  kind,
  name,
  available = true,
}: {
  session: number;
  id: string;
  kind: 'store' | 'layout';
  name: string;
  available?: boolean;
}) {
  const key = `${session}:${kind}:${id}`;
  const [image, setImage] = useState<string | null>(cache.get(key) ?? null);
  const [loading, setLoading] = useState(available && !cache.has(key));
  useEffect(() => {
    let active = true;
    const isActive = () => active;
    setImage(cache.get(key) ?? null);
    setLoading(available && !cache.has(key));
    if (available && !cache.has(key))
      void enqueue(async () => {
        try {
          for (let attempt = 0; isActive() && attempt < 12; attempt++) {
            const reply = await (kind === 'store'
              ? storePluginPreview(session, id)
              : playerLayoutPreview(session, id));
            if (!isActive()) return;
            if (reply.result === 'ok' && reply.data.mime && reply.data.data) {
              const { mime, data } = reply.data;
              if (
                !['image/png', 'image/jpeg'].includes(mime) ||
                data.length > 699052 ||
                data.length % 4 !== 0 ||
                !/^[A-Za-z0-9+/]+={0,2}$/.test(data)
              )
                break;
              const decodedSize =
                (data.length / 4) * 3 - (data.endsWith('==') ? 2 : data.endsWith('=') ? 1 : 0);
              if (decodedSize > 512 * 1024) break;
              const uri = `data:${mime};base64,${data}`;
              cache.set(key, uri);
              if (cache.size > 16) {
                const oldest = cache.keys().next().value;
                if (oldest !== undefined) cache.delete(oldest);
              }
              setImage(uri);
              return;
            }
            const pending =
              reply.result === 'ok' ? reply.data.pending === true : reply.refusal.status === 409;
            if (!pending) break;
            await new Promise((resolve) => window.setTimeout(resolve, 750));
          }
        } catch {
          /* A missing preview does not prevent plugin management. */
        } finally {
          if (active) setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [session, id, kind, key, available]);
  return (
    <div className="preview">
      {image ? (
        <img
          src={image}
          alt={name}
          onError={() => {
            cache.delete(key);
            setImage(null);
          }}
        />
      ) : (
        <span>{loading ? t('Loading preview…') : t('Preview unavailable')}</span>
      )}
    </div>
  );
}
