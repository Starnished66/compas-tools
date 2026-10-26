import { useState } from 'react';

import { useLanguage } from './i18n';
import { Sidebar, type ScreenId } from './components/Sidebar';
import { CardScreen } from './screens/card/CardScreen';
import { ActionDialog } from './screens/catalog/ActionDialog';
import { CatalogView } from './screens/catalog/CatalogView';
import { DeviceScreen } from './screens/device/DeviceScreen';
import { LibraryScreen } from './screens/library/LibraryScreen';
import { MusicScreen } from './screens/music/MusicScreen';
import { AboutScreen } from './screens/about/AboutScreen';
import { UpdatesScreen } from './screens/updates/UpdatesScreen';
import { OperationProvider } from './hooks/useOperation';
import { CardProvider, useCard } from './state/CardContext';
import { CatalogProvider } from './state/CatalogContext';
import { MusicProvider } from './state/MusicContext';

function Shell() {
  useLanguage(); // re-render everything when the language changes
  const { card } = useCard();
  const [screen, setScreen] = useState<ScreenId>('device');
  const toCard = () => {
    setScreen('card');
  };

  return (
    <div className="shell">
      <Sidebar current={screen} cardLabel={card?.label ?? null} onNavigate={setScreen} />
      <main className="main">
        {screen === 'card' && <CardScreen />}
        {(screen === 'plugins' || screen === 'layouts' || screen === 'themes') && (
          <CatalogView key={screen} view={screen} onChooseCard={toCard} />
        )}
        {screen === 'device' && <DeviceScreen />}
        {screen === 'music' && <MusicScreen onChooseCard={toCard} />}
        {screen === 'library' && <LibraryScreen onChooseCard={toCard} />}
        {screen === 'about' && <AboutScreen />}
        {screen === 'updates' && (
          <UpdatesScreen
            onGoToDevice={() => {
              setScreen('device');
            }}
          />
        )}
      </main>
      <ActionDialog />
    </div>
  );
}

export function App() {
  return (
    <CardProvider>
      <OperationProvider>
        <CatalogProvider>
          <MusicProvider>
            <Shell />
          </MusicProvider>
        </CatalogProvider>
      </OperationProvider>
    </CardProvider>
  );
}
