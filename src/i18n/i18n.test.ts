import { t } from './index';

describe('t', () => {
  it('picks plural forms and fills variables', () => {
    expect(t('{count} plugin', { count: 1 })).toBe('1 plugin');
    expect(t('{count} plugin', { count: 3 })).toBe('3 plugins');
    expect(t('Hello {name}', { name: 'Ana' })).toBe('Hello Ana');
  });

  it('shows unknown source strings as written', () => {
    expect(t('Not in the catalog')).toBe('Not in the catalog');
  });
});
