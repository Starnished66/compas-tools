import type { CardInfo, Model } from '../../api/types';
import { EjectAdvice } from '../../components/EjectAdvice';
import { Notice } from '../../components/Notice';
import { ModelPicker } from '../../components/ModelPicker';
import { formatBytes } from '../../format';
import { t } from '../../i18n';
import { LibraryStatus } from '../library/LibraryStatus';
import { useLibraryStatus } from '../library/useLibraryStatus';

interface Props {
  card: CardInfo;
  model: Model | null;
  onChooseModel: (model: Model) => void;
}

const MODEL_NAMES: Record<Model, string> = {
  r1: 'Hiby R1',
  r3proii: 'Hiby R3 Pro II',
  r3ii_2025: 'Hiby R3 II (2025)',
};

export function CardDetails({ card, model, onChooseModel }: Props) {
  const { device } = card;
  const library = useLibraryStatus();
  return (
    <section className="panel stack" aria-label={t('Player details')}>
      <h2>{card.label}</h2>
      <dl className="details-list">
        <dt>{t('Location')}</dt>
        <dd>{card.root}</dd>
        <dt>{t('File system')}</dt>
        <dd>{card.file_system || t('Unknown')}</dd>
        <dt>{t('Space')}</dt>
        <dd>
          {t('{free} free of {total}', {
            free: formatBytes(card.free_bytes),
            total: formatBytes(card.total_bytes),
          })}
        </dd>
        {device && (
          <>
            <dt>{t('Model')}</dt>
            <dd>{MODEL_NAMES[device.model]}</dd>
            <dt>{t('Player version')}</dt>
            <dd>{device.player_version}</dd>
            <dt>{t('Plugin API')}</dt>
            <dd>{device.plugin_api}</dd>
          </>
        )}
      </dl>
      {!device && (
        <Notice tone="info">
          <p>
            {t(
              'Your player has no device information yet. Update the player once, or tell Compás Tools which player it is.',
            )}
          </p>
          <ModelPicker value={model} onChange={onChooseModel} />
        </Notice>
      )}
      <LibraryStatus {...library} />
      <EjectAdvice />
    </section>
  );
}
