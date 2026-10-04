/** The words and the rules of the login screens, as the view gives them, for the tests of the screens. */

import type { LoginRules, LoginWords } from '../src/screens/login_state';

export const LOGIN_WORDS: LoginWords = {
  title: 'UOTerm',
  saved: 'Saved logins',
  no_saved: 'No saved logins yet. Fill in the form and press Save login.',
  save_login: 'Save login',
  save: 'Save',
  cancel: 'Cancel',
  save_as: 'Save as',
  saved_as: 'Saved as',
  not_saved: 'The password is not saved.',
  connect: 'Connect',
  connecting: 'Connecting...',
  pick_shard: 'Pick a shard',
  pick_character: 'Pick a character',
  make: 'New character',
  delete: 'Delete',
  delete_sure: 'Delete?',
  empty_slot: '(empty)',
  no_room: 'The account has no room for another character.',
  leave: 'Leave',
  labels: ['Host', 'Port', 'Account', 'Password', 'Shard', 'Character'],
  encryption: 'Encryption',
  encryptions: [
    ['none', 'None (most free shards)'],
    ['osi', 'OSI (encrypted shards)'],
  ],
  character_slots: 7,
};

/** The rules of the window, as far as the tests need them. */
export function rules(fault: string | null = null): LoginRules {
  return {
    fault: () => fault,
    saveName: (account, host) => `${account}@${host}`,
    detail: (account, host, port) => `${account} @ ${host}:${port}`,
    canMake: () => true,
  };
}
