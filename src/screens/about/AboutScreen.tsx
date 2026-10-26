import breezeLicence from '../../assets/icons/COPYING-ICONS?raw';
import notoLicence from '../../assets/fonts/OFL.txt?raw';
import { t } from '../../i18n';
import gpl from '../../../LICENSE?raw';
import thirdParty from '../../../THIRD_PARTY.md?raw';

/** The licences the app is distributed under, shown from the copies built into it. */
export function AboutScreen() {
  const texts = [
    { title: 'GNU General Public License, version 3', text: gpl },
    { title: t('Third-party material'), text: thirdParty },
    { title: 'Noto Sans: SIL Open Font License 1.1', text: notoLicence },
    { title: 'Breeze icons: GNU LGPL 3', text: breezeLicence },
  ];
  return (
    <section className="screen">
      <h1>{t('About')}</h1>
      <div className="panel stack">
        <p>
          {t(
            'Compás Tools is free software, released under the GNU General Public License, version 3. It comes with no warranty.',
          )}
        </p>
        <p className="muted">
          {t(
            'The installer also carries these texts in its licenses folder, together with the notices for the library code inside compas-sdtool and the exact sources it was built from.',
          )}
        </p>
      </div>
      <section className="panel stack" aria-label={t('Licences and notices')}>
        <h2>{t('Licences and notices')}</h2>
        {texts.map((entry) => (
          <details key={entry.title}>
            <summary>{entry.title}</summary>
            <pre className="release-notes">{entry.text}</pre>
          </details>
        ))}
      </section>
    </section>
  );
}
