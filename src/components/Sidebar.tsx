import { t } from '../i18n';
import { Icon } from './Icon';
import { LanguagePicker } from './LanguagePicker';

export type ScreenId =
  'card' | 'device' | 'music' | 'library' | 'plugins' | 'layouts' | 'themes' | 'updates';

interface Props {
  current: ScreenId;
  cardLabel: string | null;
  onNavigate: (screen: ScreenId) => void;
}

export function Sidebar({ current, cardLabel, onNavigate }: Props) {
  const items: { id: ScreenId; label: string }[] = [
    { id: 'card', label: t('Player') },
    { id: 'device', label: t('Device') },
    { id: 'music', label: t('Music') },
    { id: 'library', label: t('Library') },
    { id: 'plugins', label: t('Plugins') },
    { id: 'layouts', label: t('Layouts') },
    { id: 'themes', label: t('Themes') },
    { id: 'updates', label: t('Updates') },
  ];
  return (
    <nav className="sidebar" aria-label={t('Main')}>
      <div className="sidebar__brand">{t('Compás Tools')}</div>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className="nav-item"
          aria-current={item.id === current ? 'page' : undefined}
          onClick={() => {
            onNavigate(item.id);
          }}
        >
          <Icon name={item.id} />
          {item.label}
        </button>
      ))}
      <p className="sidebar__card">{cardLabel ?? t('No player selected')}</p>
      <LanguagePicker />
    </nav>
  );
}
