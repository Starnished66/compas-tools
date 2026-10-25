import { setLanguagePreference, t, useLanguage, type LanguagePreference } from '../i18n';

const NAMES: { value: LanguagePreference; label: string }[] = [
  { value: 'en', label: 'English' },
  { value: 'es', label: 'Español' },
  { value: 'de', label: 'Deutsch' },
  { value: 'fr', label: 'Français' },
  { value: 'it', label: 'Italiano' },
  { value: 'ru', label: 'Русский' },
];

/** Language names stay in their own language so anyone can find theirs. */
export function LanguagePicker() {
  const { preference } = useLanguage();
  return (
    <label className="language-picker">
      <span className="visually-hidden">{t('Language')}</span>
      <select
        className="field"
        value={preference}
        onChange={(event) => {
          setLanguagePreference(event.target.value as LanguagePreference);
        }}
      >
        <option value="system">{t('System default')}</option>
        {NAMES.map((n) => (
          <option key={n.value} value={n.value}>
            {n.label}
          </option>
        ))}
      </select>
    </label>
  );
}
