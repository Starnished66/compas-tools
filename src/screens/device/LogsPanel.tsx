import { save } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useState } from 'react';

import { collectLogs, saveLogs } from '../../api/adb';
import type { LogBundle } from '../../api/types';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { formatBytes } from '../../format';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { issueUrl } from './issue';

export function LogsPanel({ serial }: { serial: string }) {
  const operation = useOperation();
  const [bundle, setBundle] = useState<LogBundle | null>(null);
  const [savedTo, setSavedTo] = useState<string | null>(null);

  const collect = async () => {
    setSavedTo(null);
    const result = await operation.run((p) => collectLogs(serial, p));
    if (result) setBundle(result);
  };

  const saveZip = async () => {
    const path = await save({
      title: t('Save the log report'),
      defaultPath: 'compas-logs.zip',
      filters: [{ name: 'Zip', extensions: ['zip'] }],
    });
    if (!path) return;
    const done = await operation.run(() => saveLogs(path));
    if (done !== undefined) setSavedTo(path);
  };

  const included = bundle?.files.filter((f) => !f.missing) ?? [];

  return (
    <section className="panel stack" aria-label={t('Logs')}>
      <h2>{t('Logs for a bug report')}</h2>
      <p>
        {t(
          'Collects the player’s logs and basic diagnostics. PINs, tokens, Wi-Fi details, accounts and network addresses are masked before anything is saved, and nothing is uploaded.',
        )}
      </p>
      <div>
        <Button variant="primary" disabled={operation.busy} onClick={() => void collect()}>
          {t('Collect logs')}
        </Button>
      </div>
      {operation.busy && <ProgressBar progress={operation.progress} label={t('Collecting logs')} />}
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}

      {bundle && (
        <div className="stack">
          <h3>{t('This is exactly what the report will contain')}</h3>
          <ul className="log-files">
            {included.map((file) => (
              <li key={file.name}>
                <details>
                  <summary>
                    <span>{file.name}</span>
                    <span className="muted">
                      {' '}
                      {formatBytes(file.bytes)}
                      {file.redactions > 0 &&
                        ` · ${t('{count} masked', { count: file.redactions })}`}
                    </span>
                  </summary>
                  <pre className="release-notes">{file.content}</pre>
                </details>
              </li>
            ))}
          </ul>
          <p className="muted">
            {t('{count} files were not on this player and are left out.', {
              count: bundle.files.length - included.length,
            })}
          </p>
          <div className="row">
            <Button variant="primary" disabled={operation.busy} onClick={() => void saveZip()}>
              {t('Save as zip…')}
            </Button>
            <Button
              onClick={() => {
                void openUrl(issueUrl(t('Problem with my player'), bundle.summary));
              }}
            >
              {t('Open a GitHub issue')}
            </Button>
          </div>
          {savedTo && (
            <Notice tone="success">
              <p>{t('Saved to {path}. Attach it to the issue by hand.', { path: savedTo })}</p>
            </Notice>
          )}
        </div>
      )}
    </section>
  );
}
