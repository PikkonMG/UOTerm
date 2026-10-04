/**
 * The data of the Modern panels as the view gives it (`PanelData` of
 * crates/uoterm-web/src/panels.rs), and the small actions a panel sends
 * back. Every word and every color comes from the view; colors are CSS
 * (`var(--token)` of the theme, or a hue of the shard).
 */

export interface Point {
  x: number;
  y: number;
}

/** A place in the points of the panel layer: left, top, width, height. */
export interface Place {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Words and their CSS color. */
export interface Colored {
  words: string;
  color: string;
}

/** What a tooltip shows of a thing on a panel: the shard's words by serial, or plain words. */
export interface TipKey {
  serial: number | null;
  words: string;
  footer: string;
}

/** The zone of a panel a carried thing lands on. */
export type DropZone = { into: number } | 'wear' | { slot: number };

/** A small action of a panel, as its module of the view reads it. */
export type PanelAction = Record<string, unknown>;

/** Sends one action of the panel that holds it. */
export type Send = (action: PanelAction) => void;

export interface FrameData {
  /** The name of the panel in its `Panel` events. */
  panel: string;
  title: string;
  title_color: string | null;
  aside: Colored | null;
  edge: string | null;
  /** The whole panel, unfolded. */
  area: Place;
  locked: boolean;
  folded: boolean;
  foldable: boolean;
  closable: boolean;
  sizable: boolean;
}

export interface Framed<T> {
  frame: FrameData;
  body: T;
}

export interface Look {
  ui_scale: number;
  opacity: number;
}

/** The tips of the title and the marks of every frame. */
export interface FrameHints {
  drag: string;
  lock: string;
  close: string;
  size: string;
  fold: string;
}

export interface WaitingData {
  heading: Colored;
  lines: string[];
}

export interface ControlBarData {
  place: Place;
  location: { numbers: string; map_words: string; map: string; faces_words: string; facing: string };
  folded: boolean;
  launcher_words: string;
  launcher_shows: boolean;
  status: string | null;
  buttons: string[];
}

export interface ReportData {
  text: string;
  failed: boolean;
  /** The middle of the top of the words. */
  at: Point;
}

export interface ChatData {
  text: string;
  open: boolean;
  hidden: boolean;
  live: boolean;
  idle_words: string;
  mode_words: string;
  hint: string;
  pin_words: string | null;
  strip: Place | null;
}

export interface QuestionData {
  words: string;
  alarm: boolean;
  yes: string;
  no: string;
  place: Place;
  framed: FrameData | null;
}

export interface LauncherData {
  buttons: { words: string; shows: boolean }[];
}

export interface Detail {
  label: string;
  value: Colored;
}

export interface ActivityData {
  heading: string;
  idle: Colored | null;
  details: Detail[];
}

export interface VitalBar {
  label: string;
  numbers: Colored;
  fill: number;
  ghost: number;
  color: string;
  main: boolean;
}

export interface VitalsData {
  name: Colored;
  states: Colored[];
  bars: VitalBar[];
  fights: Detail | null;
}

export interface PackData {
  heading: string;
  gold: Colored;
  gold_words: string;
  rows: Detail[];
}

export interface NearRow {
  serial: number;
  name: Colored;
  title: string;
  color: string;
  hits: number | null;
  distance: number;
  hover: TipKey;
  zone: DropZone;
}

export interface NearData {
  rows: NearRow[];
  nobody: string | null;
}

export interface BarLine {
  share: number;
  color: string;
}

export interface HealthBarData {
  serial: number;
  lines: BarLine[];
  party: [string, string] | null;
  target: string | null;
  rename: string | null;
  close: string;
  zone: DropZone;
}

export interface JournalData {
  tabs: { name: string; chosen: boolean; kinds: boolean[] }[];
  kinds: string[];
  new_tab: string;
  new_tab_hint: string;
  tab_hint: string;
  tab_name_hint: string;
  rename: string;
  delete_tab: string;
  search: string;
  search_hint: string;
  save: string;
  note: Colored | null;
  filters: { words: string; shown: boolean }[];
  lines: { stamp: string | null; name: string | null; text: string; color: string }[];
  no_lines: string | null;
  back: string | null;
  glass: string;
  glass_opacity: number;
}

export type MarkData =
  | { kind: 'line'; from: Point; to: Point; width: number; color: string }
  | { kind: 'outline'; points: Point[]; width: number; color: string }
  | { kind: 'words'; at: Point; top: boolean; words: string; color: string }
  | { kind: 'dot'; at: Point; radius: number; color: string }
  | { kind: 'square'; at: Point; side: number; color: string }
  | { kind: 'ring'; at: Point; radius: number; width: number; color: string }
  | { kind: 'health_bar'; track: Place; share: number; color: string; back: string };

export interface RadarData {
  side: Point;
  land: { path: string; side: number; matrix: [number, number, number, number, number, number] };
  marks: MarkData[];
  no_files: string;
  hint: string;
}

export interface HotbarSlot {
  words: string;
  key: string;
  picture: string | null;
  tip: string;
  hover: TipKey;
}

export interface HotbarData {
  slots: HotbarSlot[];
  picking: number | null;
}

export interface PickerData {
  title: string;
  choices: string[];
  no_macros: string | null;
}

export interface Choice {
  words: string;
  chosen: boolean;
}

export interface StatRow {
  stat: number;
  name: string;
  value: string;
  lock: number;
  hover: TipKey;
}

export interface Fact {
  words: string;
  value: string;
}

export interface WornRow {
  serial: number;
  layer: number;
  picture: string | null;
  words: string;
  wear: BarLine | null;
  take_off: string;
  hover: TipKey;
}

export interface WornData {
  doll: string | null;
  name: string;
  stats: StatRow[];
  facts: Fact[];
  worn_words: string;
  rows: WornRow[];
  nothing: string | null;
  zone: DropZone;
  wear: { hint: string; button: string; on: boolean; note: Colored | null } | null;
}

export type StatusRow = ({ kind: 'title'; words: string }) | ({ kind: 'stat' } & StatRow) | ({ kind: 'fact' } & Fact);

export interface CharacterData {
  views: Choice[];
  worn: WornData | null;
  status: StatusRow[] | null;
}

export type SkillRow =
  | { kind: 'group'; at: number; name: string; words: string; fold: string; delete: string | null; hover: TipKey }
  | { kind: 'skill'; id: number; name: string; values: string[]; lock: number; buttons: [string, string] | null; hover: TipKey };

export interface SkillsData {
  sums: string;
  grouped: boolean;
  columns: { words: string; chosen: boolean; descending: boolean }[];
  new_group: string | null;
  reset: string | null;
  reset_ask: { words: string; yes: string; no: string } | null;
  rows: SkillRow[];
}

export interface SpellsData {
  books: Choice[];
  no_book: { words: string; books: [string, string][] } | null;
  empty: string | null;
  list: { id: number; name: string; icon: string | null; chosen: boolean; hover: TipKey }[];
  assign: string | null;
  detail: {
    id: number;
    icon: string | null;
    name: string;
    group: string | null;
    power: string | null;
    lines: { words: string; dim: boolean }[];
    buttons: [string, string] | null;
  } | null;
  pick: string | null;
}

export interface Member {
  serial: number;
  name: string;
  pools: [number, number, number];
  chosen: boolean;
  tell: string | null;
  kick: string | null;
  hover: TipKey;
}

export type PartyRow =
  | { kind: 'place'; number: string; member: Member | null; empty: string }
  | { kind: 'near_title'; words: string }
  | { kind: 'near'; serial: number; name: string; invite: string | null };

export interface PartyData {
  invite: { words: string; buttons: [string, string] | null } | null;
  loot: string | null;
  leave: string | null;
  add: string | null;
  rows: PartyRow[];
  tell: { hint: string; say: string } | null;
}

export interface SheetData {
  tabs: Choice[];
  live: boolean;
  character: CharacterData | null;
  skills: SkillsData | null;
  spells: SpellsData | null;
  party: PartyData | null;
}

export interface SplitData {
  amount: number;
  most: number;
  go_words: string;
}

export interface RingData {
  center: Point;
  name: string;
  lines: { words: string; enabled: boolean; at: Point }[];
}

export interface TooltipData {
  lines: string[];
  footer: string;
}

export interface CarriedData {
  picture: string | null;
  words: string;
  /** How opaque the carried picture shows. */
  alpha: number;
}

export interface PanelData {
  look: Look;
  hints: FrameHints;
  title: string;
  waiting: WaitingData | null;
  alarm: number;
  bar: ControlBarData | null;
  launcher: Framed<LauncherData> | null;
  activity: Framed<ActivityData> | null;
  vitals: Framed<VitalsData> | null;
  pack: Framed<PackData> | null;
  near: Framed<NearData> | null;
  bars: Framed<HealthBarData>[];
  journal: Framed<JournalData> | null;
  radar: Framed<RadarData> | null;
  hotbar: Framed<HotbarData> | null;
  picker: PickerData | null;
  sheet: Framed<SheetData> | null;
  split: Framed<SplitData> | null;
  ring: RingData | null;
  report: ReportData | null;
  question: QuestionData | null;
  chat: ChatData;
  tooltip: TooltipData | null;
  carried: CarriedData | null;
}

/** A name plate as the view lays it out (`PlacedPlate`): colors are premultiplied RGBA bytes. */
export interface PlacedPlate {
  area: { min: Point; max: Point };
  name_at: Point;
  name: string;
  name_color: [number, number, number, number];
  bar: { back: { min: Point; max: Point }; fill: { min: Point; max: Point }; color: [number, number, number, number] } | null;
}

/** Words over a head (`PlacedWords`): the middle of their bottom at `x`, `y`. */
export interface PlacedWords {
  words: string;
  x: number;
  y: number;
  color: [number, number, number, number];
  alpha: number;
  number: boolean;
}
