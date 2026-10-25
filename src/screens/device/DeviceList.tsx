import type { DeviceView } from '../../api/types';
import { Button } from '../../components/Button';
import { t } from '../../i18n';
import { BOARD_NAMES, firmwareLabel } from './labels';

interface Props {
  devices: DeviceView[];
  selected: string | null;
  scanning: boolean;
  onSelect: (serial: string) => void;
  onScan: () => void;
}

function describe(device: DeviceView): string {
  if (device.state === 'unauthorized') return t('Allow this computer on the player’s screen.');
  if (device.state !== 'device') return t('Not ready ({state}).', { state: device.state });
  const { info } = device;
  if (!info) return t('Could not read the player.');
  const board = info.board ? BOARD_NAMES[info.board] : t('Unknown model');
  const version = info.player_version ? ` · ${info.player_version}` : '';
  return `${board} · ${firmwareLabel(info.firmware)}${version}`;
}

export function DeviceList({ devices, selected, scanning, onSelect, onScan }: Props) {
  return (
    <section className="stack" aria-label={t('Connected players')}>
      <div className="row">
        <h2>{t('Connected players')}</h2>
        <Button disabled={scanning} onClick={onScan}>
          {t('Scan again')}
        </Button>
      </div>
      {devices.length === 0 ? (
        <p className="muted">
          {scanning
            ? t('Looking for players…')
            : t('No player found. Switch ADB on and connect the USB cable.')}
        </p>
      ) : (
        <ul className="card-list">
          {devices.map((device) => (
            <li
              key={device.serial}
              className={`card-list__item ${device.serial === selected ? 'card-list__item--selected' : ''}`}
            >
              <div>
                <strong>{device.serial}</strong>
                <div className="muted">{describe(device)}</div>
              </div>
              <Button
                variant={device.serial === selected ? 'secondary' : 'primary'}
                disabled={device.state !== 'device' || device.serial === selected}
                aria-label={t('Use {name}', { name: device.serial })}
                onClick={() => {
                  onSelect(device.serial);
                }}
              >
                {device.serial === selected ? t('In use') : t('Use this player')}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
