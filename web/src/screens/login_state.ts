/**
 * The login screens with no drawing: which screen a question of the login
 * shows, the replies the login link reads, the character a reply plays,
 * and where the profile of that character is kept.
 */

import type { LoginAsk, LoginReply, NewCharacterWish } from '../net/login';

/** The screen of a question of the login. */
export type AskScreen = 'picking' | 'characters';

/** The words of the login screens, as the view gives them (`loginWords()`). */
export interface LoginWords {
  title: string;
  saved: string;
  no_saved: string;
  save_login: string;
  save: string;
  cancel: string;
  save_as: string;
  saved_as: string;
  not_saved: string;
  connect: string;
  connecting: string;
  pick_shard: string;
  pick_character: string;
  make: string;
  delete: string;
  delete_sure: string;
  empty_slot: string;
  no_room: string;
  leave: string;
  /** The fields of the form, in their order: host, port, account, password, shard, character. */
  labels: string[];
  encryption: string;
  /** Each encryption by its wire name, and its words. */
  encryptions: [string, string][];
  /** The slots the character list shows. */
  character_slots: number;
}

/** The rules of the login screens, as the view gives them. */
export interface LoginRules {
  /** Why a form cannot log in, or null; a form to save gives no password (null). */
  fault(host: string, port: string, account: string, password: string | null): string | null;
  /** The name "Save login" offers. */
  saveName(account: string, host: string): string;
  /** The line under the name of a saved login. */
  detail(account: string, host: string, port: number): string;
  /** True when the account may make one more character. */
  canMake(names: string[], listFlags: number): boolean;
}

/** One character of one shard: whose profile the game keeps. */
export interface CharacterPlace {
  /** The login server, `host:port`. */
  shard: string;
  character: string;
}

/** A session the address names, and its character when the address names one. */
export interface SessionPlace {
  session: string;
  character: CharacterPlace | null;
}

const PROFILES_PATH = '/v1/profiles';
/** Where the profile of every character is kept until the page knows the shard and the character. */
export const DEFAULT_PROFILE_PATH = `${PROFILES_PATH}/default`;
const PATH_SEPARATOR = '/';
const SOUND_FONT_PATH = '/v1/soundfont';
const SESSION_QUERY = 'session';
const SHARD_QUERY = 'shard';
const CHARACTER_QUERY = 'character';
const PORT_SEPARATOR = ':';

export function nextScreen(ask: LoginAsk): AskScreen {
  return ask.kind === 'Shard' ? 'picking' : 'characters';
}

export const pick = (index: number): LoginReply => ({ kind: 'Pick', index });
export const play = (slot: number): LoginReply => ({ kind: 'Request', request: { Play: slot } });
export const remove = (slot: number): LoginReply => ({ kind: 'Request', request: { Delete: slot } });
export const make = (wish: NewCharacterWish): LoginReply => ({ kind: 'Request', request: { Make: wish } });
export const leave = (): LoginReply => ({ kind: 'Request', request: 'Leave' });

/** The character a reply to the character list plays; null for a reply that plays none. */
export function characterOf(reply: LoginReply, names: string[]): string | null {
  if (reply.kind !== 'Request' || typeof reply.request === 'string') return null;
  const request = reply.request;
  if ('Make' in request) return request.Make.name;
  if ('Play' in request) return names[request.Play] || null;
  return null;
}

/** The login server of a host and a port, as profiles name a shard. */
export function shardOf(host: string, port: number): string {
  return `${host.trim()}${PORT_SEPARATOR}${port}`;
}

/** Where the MIDI sound font the profile of a character names comes from; that of the default profile with none. */
export function soundFontPath(place: CharacterPlace | null): string {
  if (place === null) return SOUND_FONT_PATH;
  return `${SOUND_FONT_PATH}?${new URLSearchParams({ [SHARD_QUERY]: place.shard, [CHARACTER_QUERY]: place.character })}`;
}

/** Where the profile of a character is kept; the default profile with none. */
export function profilePath(place: CharacterPlace | null): string {
  if (place === null) return DEFAULT_PROFILE_PATH;
  return [PROFILES_PATH, encodeURIComponent(place.shard), encodeURIComponent(place.character)].join(PATH_SEPARATOR);
}

/** The query of the address that opens a session again on a reload, with the character it plays. */
export function sessionSearch(session: string, place: CharacterPlace | null): string {
  const query = new URLSearchParams({ [SESSION_QUERY]: session });
  if (place) {
    query.set(SHARD_QUERY, place.shard);
    query.set(CHARACTER_QUERY, place.character);
  }
  return `?${query}`;
}

/** The session the query of the address names, with its character; null with no session. */
export function readPlace(query: URLSearchParams): SessionPlace | null {
  const session = query.get(SESSION_QUERY);
  if (session === null) return null;
  const shard = query.get(SHARD_QUERY);
  const character = query.get(CHARACTER_QUERY);
  return { session, character: shard && character ? { shard, character } : null };
}
