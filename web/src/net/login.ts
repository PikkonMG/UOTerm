/**
 * The login link: the page sends the login, answers each question of the
 * login, and gets the id of the new session, or the words of the fault.
 * The password goes out in the first message only.
 */

import { socketUrl } from './api';

const LOGIN_PATH = '/v1/login/live';
export const LOGIN_CLOSED = 'the login link closed before the login ended';

/** Where to log in and as whom; the server fills what is left out from its config. */
export interface LoginForm {
  host: string;
  port: number;
  account: string;
  password: string;
  /** The shard of the list. None or blank: the page picks when asked. */
  shard?: string | null;
  /** The character to play. None or blank: the page picks when asked. */
  character?: string | null;
  era?: string | null;
  version?: string | null;
  encryption?: 'none' | 'osi';
}

export interface TownPlace {
  x: number;
  y: number;
  z: number;
  map: number;
  description: number;
}

export interface StartTown {
  index: number;
  name: string;
  building: string;
  /** Where it is; an older list names no place. */
  place: TownPlace | null;
}

/** What the shard lets a new character be. */
export interface CharacterChoices {
  towns: StartTown[];
  features: number;
  list_flags: number;
}

export interface NewCharacterWish {
  name: string;
  female: boolean;
  race: number;
  strength: number;
  dexterity: number;
  intelligence: number;
  /** [skill, value] pairs. */
  skills: [number, number][];
  skin_hue: number;
  hair: number;
  hair_hue: number;
  beard: number;
  beard_hue: number;
  shirt_hue: number;
  pants_hue: number;
  profession: number;
  start_city: number;
  slot: number;
}

/** What to do with the characters of the account. */
export type CharacterRequest = { Play: number } | { Delete: number } | { Make: NewCharacterWish } | 'Leave';

/** A question of the login. */
export type LoginAsk =
  | { kind: 'Shard'; names: string[] }
  | { kind: 'Characters'; names: string[]; refused: string | null; choices: CharacterChoices };

/** The answer to a `LoginAsk`. */
export type LoginReply = { kind: 'Pick'; index: number } | { kind: 'Request'; request: CharacterRequest };

type LoginIn = { kind: 'ask'; ask: LoginAsk } | { kind: 'ready'; session: string } | { kind: 'failed'; words: string };

/** The login did not end in a session: the words say why. */
export class LoginFailed extends Error {
  constructor(words: string) {
    super(words);
    this.name = 'LoginFailed';
  }
}

/**
 * Logs in with `form`, and asks `onAsk` each question of the login. Gives
 * the id of the new session; rejects with `LoginFailed`, or with the error
 * of `onAsk`, which closes the link and so ends the login.
 */
export function login(form: LoginForm, onAsk: (ask: LoginAsk) => Promise<LoginReply>): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(socketUrl(LOGIN_PATH));
    let ended = false;
    const end = (finish: () => void) => {
      if (ended) return;
      ended = true;
      socket.onclose = null;
      socket.onmessage = null;
      socket.close();
      finish();
    };
    const answer = (ask: LoginAsk) =>
      onAsk(ask).then(
        (reply) => {
          if (!ended) socket.send(JSON.stringify({ kind: 'reply', reply }));
        },
        (error: unknown) => end(() => reject(error)),
      );
    socket.onopen = () => socket.send(JSON.stringify({ kind: 'login', ...form }));
    socket.onclose = () => end(() => reject(new LoginFailed(LOGIN_CLOSED)));
    socket.onmessage = (event: MessageEvent) => {
      const message = JSON.parse(event.data as string) as LoginIn;
      switch (message.kind) {
        case 'ask':
          void answer(message.ask);
          break;
        case 'ready':
          end(() => resolve(message.session));
          break;
        case 'failed':
          end(() => reject(new LoginFailed(message.words)));
          break;
      }
    };
  });
}
