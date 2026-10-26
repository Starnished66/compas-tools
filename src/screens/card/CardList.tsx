import type { CardInfo } from '../../api/types';
import { Button } from '../../components/Button';
import { t } from '../../i18n';

interface Props {
  cards: CardInfo[];
  selectedRoot: string | null;
  scanning: boolean;
  onSelect: (card: CardInfo) => void;
}

export function CardList({ cards, selectedRoot, scanning, onSelect }: Props) {
  if (cards.length === 0) {
    return (
      <p className="muted">
        {scanning
          ? t('Looking for players…')
          : t('No player found. Connect your player, or choose its folder.')}
      </p>
    );
  }
  return (
    <ul className="card-list" aria-label={t('Connected players')}>
      {cards.map((card) => {
        const selected = card.root === selectedRoot;
        return (
          <li
            key={card.root}
            className={`card-list__item ${selected ? 'card-list__item--selected' : ''}`}
          >
            <div>
              <strong>{card.label}</strong>
              <div className="muted">{card.root}</div>
            </div>
            <Button
              variant={selected ? 'secondary' : 'primary'}
              disabled={selected}
              aria-label={t('Use {name}', { name: card.label })}
              onClick={() => {
                onSelect(card);
              }}
            >
              {selected ? t('In use') : t('Use this player')}
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
