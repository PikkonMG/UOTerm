/**
 * The creation of a new character as the page sees it: the methods of the
 * `CreationView` of the WebAssembly, and the values it gives. The view
 * runs every rule; the page draws what `screen()` gives and calls the
 * view on each click.
 */

import type { FeedView } from '../net/art';
import type { NewCharacterWish } from '../net/login';

/** A picture the page has, by the key the art feed keeps its pixels under. */
export interface ShownPicture {
  key: string;
  width: number;
  height: number;
  anchor_x: number;
  anchor_y: number;
}

export type Progress = 'Done' | 'Current' | 'Ahead';
export type RgbColor = [number, number, number];

export interface StagePill {
  words: string;
  progress: Progress;
}

export interface Preview {
  figure: ShownPicture | null;
  title: string;
  about: string;
}

export interface Footer {
  words: string;
  blocked: boolean;
  next: string;
}

export interface Chip {
  words: string;
  picked: boolean;
}

export interface RaceChoice {
  race: string;
  words: string;
  picked: boolean;
  locked: boolean;
}

export interface ColorGrid {
  paint: string;
  words: string;
  rows: number;
  columns: number;
  cells: (RgbColor | null)[];
  picked: number;
}

export interface Card {
  true_name: string;
  name: string;
  about: string;
  short: string;
  locked: boolean;
  picked: boolean;
  picture: ShownPicture | null;
}

export interface PointsRow {
  words: string;
  picked: boolean;
  value: number;
}

export interface PointsGroup {
  total: string;
  whole: boolean;
  rule: string;
  limit: string | null;
  least: number;
  most: number;
  rows: PointsRow[];
}

export interface TownRow {
  name: string;
  building: string;
  facet: string | null;
  picked: boolean;
}

export interface TownMap {
  /** The picture of the land round the town, the town in its middle. */
  path: string;
  /** The tiles on a side of the picture. */
  span: number;
  /** The tiles across the map the page shows. */
  tiles: number;
}

export interface SummaryRow {
  label: string;
  value: string;
}

export interface Swatch {
  words: string;
  color: RgbColor | null;
}

export interface LookPage {
  step: 'Look';
  female: boolean;
  races: RaceChoice[];
  hair: Chip[];
  beards: Chip[] | null;
  colors: ColorGrid[];
}

export interface ProfessionPage {
  step: 'Profession';
  cards: Card[];
  details: string[];
}

export interface TradePage {
  step: 'Trade';
  stats: PointsGroup;
  skills: PointsGroup;
}

export interface TownPage {
  step: 'Town';
  towns: TownRow[];
  map: TownMap | null;
  words: string;
}

export interface NamePage {
  step: 'Name';
  name: string;
  verdict: string;
  good: boolean;
  rules: string[];
  summary: SummaryRow[];
  swatches: Swatch[];
}

export type Page = LookPage | ProfessionPage | TradePage | TownPage | NamePage;

export interface CreationScreen {
  /** False while the creation files are on their way. */
  ready: boolean;
  stages: StagePill[];
  preview: Preview;
  footer: Footer;
  page: Page;
}

/** One skill a row picks from. */
export interface SkillChoice {
  skill: number;
  words: string;
  taken: boolean;
  picked: boolean;
}

/** The fixed words of the creation screen, as the view gives them (`creationWords()`). */
export interface CreationWords {
  title: string;
  back: string;
  keys: string;
  turn_left: string;
  turn_right: string;
  no_art: string;
  body: string;
  male: string;
  female: string;
  hair: string;
  beard: string;
  profession: string;
  profession_hint: string;
  stats: string;
  skills: string;
  pick_skill: string;
  search: string;
  town: string;
  name: string;
  name_hint: string;
  summary: string;
  colors: string;
  loading: string;
}

/** The methods of the `CreationView` the creation screen calls. */
export interface CreationModel extends FeedView {
  screen(): CreationScreen;
  setFemale(female: boolean): void;
  setRace(race: string): void;
  setHair(at: number): void;
  setBeard(at: number): void;
  setColor(paint: string, at: number): void;
  pickProfession(trueName: string): void;
  setStat(at: number, value: number): void;
  setSkill(row: number, skill: number): void;
  setSkillValue(row: number, value: number): void;
  skillChoices(row: number, search: string): SkillChoice[];
  setTown(at: number): void;
  setName(name: string): void;
  turn(right: boolean): void;
  /** One page on; true when the character is made. */
  next(): boolean;
  /** One page back; true when the player left the creation. */
  back(): boolean;
  previewScale(roomWidth: number, roomHeight: number, artWidth: number, artHeight: number): number;
}

/** The `CreationView` with the wish it sends, which the login asks for. */
export interface CreationMaker extends CreationModel {
  wish(namesJson: string): NewCharacterWish;
  free(): void;
}
