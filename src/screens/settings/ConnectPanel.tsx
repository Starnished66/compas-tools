import { useState } from 'react';

import { Button } from '../../components/Button';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import type { Problem } from './useSettings';

interface Props {
  problem: Problem;
  error: string | null;
  /** The PIN is handed over once; this screen forgets it at once. */
  onConnect: (pin?: string) => void;
}

function heading(problem: Problem): string {
  switch (problem) {
    case 'unsupported':
      return t('This player does not offer remote settings');
    case 'busy':
      return t('The player is busy, trying again');
    case 'rate_limited':
      return t('Too many wrong PIN attempts, wait a moment');
    case 'pin':
    case 'wrong_pin':
      return t('Enter the player’s PIN');
    default:
      return t('Remote Control is not reachable on the player');
  }
}

/** What is missing before the settings can open: Remote Control on, then the PIN. */
export function ConnectPanel({ problem, error, onConnect }: Props) {
  const [pin, setPin] = useState('');
  const submit = () => {
    // An empty field means: use the PIN already held, if any.
    onConnect(pin === '' ? undefined : pin);
    setPin('');
  };
  return (
    <EmptyState title={heading(problem)}>
      {error && <ErrorBanner message={error} />}
      {problem === 'wrong_pin' && (
        <ErrorBanner message={t('The player did not accept that PIN.')} />
      )}
      {problem === 'busy' || problem === 'rate_limited' ? null : problem === 'unsupported' ? (
        <p>
          {t(
            'The Compás player on this device is too old for this. Update it from Installation and Updates, then come back.',
          )}
        </p>
      ) : (
        <>
          <p>
            {t(
              'Compás Tools changes the settings through the player’s Remote Control, which is off every time the player starts. It has to be turned on on the player itself.',
            )}
          </p>
          <ol className="steps">
            <li>
              {t('On the player, open the Remote Control screen and turn Remote Control on.')}
            </li>
            <li>{t('Type the PIN shown there. It is kept only while this page is open.')}</li>
          </ol>
          <form
            className="row"
            onSubmit={(event) => {
              event.preventDefault();
              submit();
            }}
          >
            <label className="row">
              <span>{t('PIN')}</span>
              <input
                className="field"
                type="password"
                inputMode="numeric"
                autoComplete="off"
                value={pin}
                onChange={(event) => {
                  setPin(event.target.value);
                }}
              />
            </label>
            <Button type="submit" variant="primary">
              {problem === 'pin' || problem === 'wrong_pin' ? t('Connect') : t('Retry')}
            </Button>
          </form>
        </>
      )}
      {(problem === 'unsupported' || problem === 'busy' || problem === 'rate_limited') && (
        <Button onClick={submit}>{t('Retry')}</Button>
      )}
    </EmptyState>
  );
}
