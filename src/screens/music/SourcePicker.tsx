import { open } from '@tauri-apps/plugin-dialog';

import { errorMessage } from '../../api/invoke';
import { Button } from '../../components/Button';
import { MiddleEllipsis } from '../../components/MiddleEllipsis';
import { t } from '../../i18n';

interface Props {
  sources: string[];
  hovering: boolean;
  disabled: boolean;
  onAdd: (paths: string[]) => void;
  onRemove: (path: string) => void;
  onError: (message: string) => void;
}

export function SourcePicker({ sources, hovering, disabled, onAdd, onRemove, onError }: Props) {
  const pick = async (directory: boolean) => {
    try {
      const chosen = await open({ multiple: true, directory });
      if (chosen) onAdd(chosen);
    } catch (failure) {
      onError(errorMessage(failure));
    }
  };

  return (
    <section
      className={`dropzone ${hovering ? 'dropzone--hover' : ''}`}
      aria-label={t('Music to copy')}
    >
      <p>{t('Drop music files or folders here, or choose them.')}</p>
      <div className="row">
        <Button disabled={disabled} onClick={() => void pick(false)}>
          {t('Add files…')}
        </Button>
        <Button disabled={disabled} onClick={() => void pick(true)}>
          {t('Add folders…')}
        </Button>
      </div>
      {sources.length > 0 && (
        <ul className="source-list">
          {sources.map((path) => (
            <li key={path}>
              <span className="source-list__path">
                <MiddleEllipsis text={path} />
              </span>
              <Button
                disabled={disabled}
                aria-label={t('Remove {path}', { path })}
                onClick={() => {
                  onRemove(path);
                }}
              >
                {t('Remove')}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
