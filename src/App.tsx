import { useEffect, useState } from 'react';

import { useLanguage } from './i18n';
import { Sidebar, type ScreenId } from './components/Sidebar';
import { AboutScreen } from './screens/about/AboutScreen';
import { ActionDialog } from './screens/catalog/ActionDialog';
import { CatalogView } from './screens/catalog/CatalogView';
import { BOARD_NAMES } from './screens/install/labels';
import { InstallScreen } from './screens/install/InstallScreen';
import { LibraryScreen } from './screens/library/LibraryScreen';
import { PlayerSettingsScreen } from './screens/settings/PlayerSettingsScreen';
import { OperationProvider } from './hooks/useOperation';
import { t } from './i18n';
import { CardProvider, useCard } from './state/CardContext';
import { CatalogProvider } from './state/CatalogContext';
import { ModeProvider, useMode } from './state/ModeContext';
import { MusicProvider } from './state/MusicContext';

/** The player in use and how it is connected: its drive first, else the one seen over ADB. */
function usePlayerLabel(): string | null {
  const { card, model } = useCard();
  const { adbPlayers } = useMode();
  if (card) {
    return `${model ? BOARD_NAMES[model] : card.label} · ${t('Storage mode')}`;
  }
  const [adb] = adbPlayers;
  if (adb) {
    return `${adb.board ? BOARD_NAMES[adb.board] : t('Unknown model')} · ${t('ADB mode')}`;
  }
  return null;
}

function Shell() {
  useLanguage(); // re-render everything when the language changes
  const [screen, setScreen] = useState<ScreenId>('install');
  const { rescan } = useMode();
  const playerLabel = usePlayerLabel();

  // A cable may have been moved or a mode switched since the last screen was open.
  useEffect(() => {
    void rescan();
  }, [screen, rescan]);

  return (
    <div className="shell">
      <Sidebar current={screen} playerLabel={playerLabel} onNavigate={setScreen} />
      <main className="main">
        {screen === 'install' && <InstallScreen />}
        {screen === 'library' && <LibraryScreen />}
        {(screen === 'plugins' || screen === 'layouts' || screen === 'themes') && (
          <CatalogView key={screen} view={screen} />
        )}
        {screen === 'player-settings' && <PlayerSettingsScreen />}
        {screen === 'about' && <AboutScreen />}
      </main>
      <ActionDialog />
    </div>
  );
}

export function App() {
  return (
    <CardProvider>
      <ModeProvider>
        <OperationProvider>
          <CatalogProvider>
            <MusicProvider>
              <Shell />
            </MusicProvider>
          </CatalogProvider>
        </OperationProvider>
      </ModeProvider>
    </CardProvider>
  );
}
