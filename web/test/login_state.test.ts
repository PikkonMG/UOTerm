import { describe, expect, it } from 'vitest';
import { WISH } from './fake_creation';
import { characterOf, leave, make, nextScreen, pick, play, profilePath, readPlace, remove, sessionSearch, soundFontPath } from '../src/screens/login_state';

const NO_CHOICES = { towns: [], features: 0, list_flags: 0 };
const MARA = { ...WISH, name: 'Mara' };

describe('login state', () => {
  it('shows_the_character_list_for_a_characters_question', () => {
    expect(nextScreen({ kind: 'Characters', names: ['Mara'], refused: null, choices: NO_CHOICES })).toBe('characters');
  });

  it('shows_the_shard_list_for_a_shard_question', () => {
    expect(nextScreen({ kind: 'Shard', names: ['Atlantic'] })).toBe('picking');
  });

  it('builds_the_wire_replies_the_server_reads', () => {
    expect(play(2)).toEqual({ kind: 'Request', request: { Play: 2 } });
    expect(remove(1)).toEqual({ kind: 'Request', request: { Delete: 1 } });
    expect(pick(0)).toEqual({ kind: 'Pick', index: 0 });
    expect(make(MARA)).toEqual({ kind: 'Request', request: { Make: MARA } });
    expect(leave()).toEqual({ kind: 'Request', request: 'Leave' });
  });

  it('names_the_character_a_reply_plays', () => {
    const names = ['', 'Mara'];
    expect(characterOf(play(1), names)).toBe('Mara');
    expect(characterOf(make(MARA), names)).toBe('Mara');
    expect(characterOf(remove(1), names)).toBeNull();
    expect(characterOf(play(5), names)).toBeNull();
  });

  it('keeps_the_profile_of_the_character_of_the_shard', () => {
    expect(profilePath({ shard: '127.0.0.1:2593', character: 'Mara Dell' })).toBe('/v1/profiles/127.0.0.1%3A2593/Mara%20Dell');
    expect(profilePath(null)).toBe('/v1/profiles/default');
  });

  it('asks_for_the_sound_font_the_profile_of_the_character_names', () => {
    expect(soundFontPath({ shard: '127.0.0.1:2593', character: 'Mara Dell' })).toBe('/v1/soundfont?shard=127.0.0.1%3A2593&character=Mara+Dell');
    expect(soundFontPath(null)).toBe('/v1/soundfont');
  });

  it('carries_the_session_and_its_character_in_the_address_for_a_reload', () => {
    const search = sessionSearch('s1', { shard: 'play.example.com:2593', character: 'Mara' });
    expect(readPlace(new URLSearchParams(search))).toEqual({
      session: 's1',
      character: { shard: 'play.example.com:2593', character: 'Mara' },
    });
    expect(readPlace(new URLSearchParams('?session=s2'))).toEqual({ session: 's2', character: null });
    expect(readPlace(new URLSearchParams(''))).toBeNull();
  });
});
