import { render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Game } from '../src/screens/Game';

const START_FAULT = 'no WebGL';

vi.mock('../src/game', () => ({
  startGame: () => {
    throw new Error(START_FAULT);
  },
}));

describe('Game', () => {
  it('tells_the_fault_of_a_game_that_did_not_start', () => {
    const onFault = vi.fn();
    const profile = { path: '/v1/profiles/default', value: {}, soundFont: '/v1/sound-font' };
    render(<Game session="s1" profile={profile} onEnded={vi.fn()} onFault={onFault} />);
    expect(onFault).toHaveBeenCalledWith(START_FAULT);
  });
});
