import { t } from '../i18n';
import { Icon } from './Icon';
import { LanguagePicker } from './LanguagePicker';
import { SupportLink } from './SupportLink';

export type ScreenId = 'install' | 'library' | 'plugins' | 'layouts' | 'themes' | 'about';

interface Props {
  current: ScreenId;
  /** The player in use and how it is connected, e.g. "HiBy R1 · ADB mode". */
  playerLabel: string | null;
  onNavigate: (screen: ScreenId) => void;
}

export function Sidebar({ current, playerLabel, onNavigate }: Props) {
  const items: { id: ScreenId; label: string; icon: 'updates' | 'library' | ScreenId }[] = [
    { id: 'install', label: t('Installation and Updates'), icon: 'updates' },
    { id: 'library', label: t('Music Library'), icon: 'library' },
    { id: 'plugins', label: t('Plugins'), icon: 'plugins' },
    { id: 'layouts', label: t('Layouts'), icon: 'layouts' },
    { id: 'themes', label: t('Themes'), icon: 'themes' },
    { id: 'about', label: t('About'), icon: 'about' },
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
          <Icon name={item.icon} />
          {item.label}
        </button>
      ))}
      <p className="sidebar__card">{playerLabel ?? t('No player selected')}</p>
      <SupportLink className="sidebar__support" />
      <LanguagePicker />
    </nav>
  );
}
