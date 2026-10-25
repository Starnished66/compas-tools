import { finishingSteps } from './instructions';

describe('finishingSteps', () => {
  const recovery = (model: 'r1' | 'r3proii' | 'r3ii_2025') =>
    finishingSteps('full_image', model).find((s) => s.includes('recovery'));

  it('names the recovery keys of each board', () => {
    expect(recovery('r1')).toContain('Power + Volume Up');
    expect(recovery('r3proii')).toContain('Power + Volume Up');
    expect(recovery('r3ii_2025')).toContain('Power + Play/Pause');
  });

  it('leaves recovery out when the model is unknown, and for player-only updates', () => {
    expect(finishingSteps('full_image', null).join(' ')).not.toContain('recovery');
    expect(finishingSteps('player_only', 'r1').join(' ')).not.toContain('recovery');
  });
});
