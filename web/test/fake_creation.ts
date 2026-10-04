/** A creation view on its name page, whose only rule is the length of the name, for the tests of the screens. */

import { vi } from 'vitest';
import type { CreationMaker, CreationScreen, CreationWords, NamePage } from '../src/screens/creation_model';
import type { NewCharacterWish } from '../src/net/login';

/** A new character as the fake sends it, under the name typed. */
export const WISH: NewCharacterWish = {
  name: '',
  female: false,
  race: 0,
  strength: 60,
  dexterity: 15,
  intelligence: 15,
  skills: [[1, 30]],
  skin_hue: 1002,
  hair: 0x203b,
  hair_hue: 1102,
  beard: 0,
  beard_hue: 0,
  shirt_hue: 3,
  pants_hue: 3,
  profession: 0,
  start_city: 0,
  slot: 1,
};

export const SHORT = 'The name needs at least 2 letters.';
export const GOOD = 'The name is good.';
export const CREATE = 'Create';
export const NEXT = 'Next';

export const CREATION_WORDS: CreationWords = {
  title: 'New character',
  back: 'Back',
  keys: 'Enter: next     Esc: back',
  turn_left: 'Turn left',
  turn_right: 'Turn right',
  no_art: 'The figure needs the client files.',
  body: 'Body',
  male: 'Male',
  female: 'Female',
  hair: 'Hair',
  beard: 'Beard',
  profession: 'Pick a profession',
  profession_hint: 'Or pick Custom to set your own stats and skills.',
  stats: 'Stats',
  skills: 'Skills',
  pick_skill: 'Pick a skill',
  search: 'Search skills',
  town: 'Pick a start town',
  name: 'Name your character',
  name_hint: 'Type a name',
  summary: 'Your character',
  colors: 'Colors',
  loading: 'Reading the client files...',
};

/** The faults of a name, as the view gives them: only the length here. */
function nameFault(name: string): string | null {
  return name.length < 2 ? SHORT : null;
}

/** A view on the name page whose rules are only the name length. */
export function namePageView(): CreationMaker & { setName: ReturnType<typeof vi.fn> } {
  let name = '';
  const screen = (): CreationScreen => {
    const fault = nameFault(name);
    const page: NamePage = {
      step: 'Name',
      name,
      verdict: fault ?? GOOD,
      good: fault === null,
      rules: ['2 to 16 characters.'],
      summary: [{ label: 'Look', value: 'Human man' }],
      swatches: [],
    };
    return {
      ready: true,
      stages: [{ words: 'Name & confirm', progress: 'Current' }],
      preview: { figure: null, title: name || 'Human man', about: '' },
      footer: { words: fault ?? CREATION_WORDS.keys, blocked: fault !== null, next: CREATE },
      page,
    };
  };
  return {
    screen,
    setName: vi.fn((typed: string) => {
      name = typed;
    }),
    setFemale: vi.fn(),
    setRace: vi.fn(),
    setHair: vi.fn(),
    setBeard: vi.fn(),
    setColor: vi.fn(),
    pickProfession: vi.fn(),
    setStat: vi.fn(),
    setSkill: vi.fn(),
    setSkillValue: vi.fn(),
    skillChoices: vi.fn(() => []),
    setTown: vi.fn(),
    turn: vi.fn(),
    next: vi.fn(() => nameFault(name) === null),
    back: vi.fn(() => false),
    previewScale: vi.fn(() => 1),
    wish: vi.fn(() => ({ ...WISH, name })),
    free: vi.fn(),
    artWanted: () => [],
    artArrived: () => {},
    artMissing: () => {},
    artForgotten: () => [],
    dataWanted: () => [],
    dataArrived: () => {},
    dataMissing: () => {},
    postsWanted: () => [],
    postArrived: () => {},
    postMissing: () => {},
  };
}

