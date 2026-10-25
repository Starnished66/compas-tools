import { useEffect, useState } from 'react';

import { loadPreviewUrl } from '../../api/catalog';
import { t } from '../../i18n';

interface Props {
  pluginId: string;
  available: boolean;
}

/** The verified preview image, loaded when the card appears. */
export function Preview({ pluginId, available }: Props) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    if (!available) return;
    let cancelled = false;
    let created: string | null = null;
    loadPreviewUrl(pluginId)
      .then((value) => {
        if (cancelled) URL.revokeObjectURL(value);
        else {
          created = value;
          setUrl(value);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [pluginId, available]);

  return (
    <div className="preview">
      {url ? (
        <img src={url} alt="" />
      ) : (
        <span>{available ? t('Loading preview…') : t('No preview')}</span>
      )}
    </div>
  );
}
