import { useState } from 'react';

import { Sidebar, type ScreenId } from './components/Sidebar';
import { CardScreen } from './screens/card/CardScreen';
import { CatalogView } from './screens/catalog/CatalogView';
import { UpdatesScreen } from './screens/updates/UpdatesScreen';
import { CardProvider, useCard } from './state/CardContext';
import { CatalogProvider } from './state/CatalogContext';

function Shell() {
  const { card } = useCard();
  const [screen, setScreen] = useState<ScreenId>('card');
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
        {screen === 'updates' && <UpdatesScreen onChooseCard={toCard} />}
      </main>
    </div>
  );
}

export function App() {
  return (
    <CardProvider>
      <CatalogProvider>
        <Shell />
      </CatalogProvider>
    </CardProvider>
  );
}
