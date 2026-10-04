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
  /** A thing of a grid: the view adds the compare and the bag lines. */
  in_grid?: boolean;
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

/** A button of the head of a grid. */
export interface TitleButton {
  words: string;
  color: string;
  hint: string;
}

export interface GridItem {
  serial: number;
  picture: string | null;
  alpha: number;
  amount: Colored | null;
  mark: string | null;
  chosen: boolean;
  /** The share of the slider of a pile of a grid loot. */
  slider: number | null;
  hover: TipKey;
  zone: DropZone;
}

export interface GridCell {
  slot: number;
  locked: boolean;
  item: GridItem | null;
}

/** A grid container (`GridData`). */
export interface GridData {
  serial: number;
  live: boolean;
  glass: string;
  glass_opacity: number;
  search: string;
  search_hint: string;
  count: string;
  favorite: TitleButton | null;
  loot_all: TitleButton | null;
  loot_bag: TitleButton | null;
  columns: number;
  side: number;
  gap: number;
  art_scale: number;
  cells: GridCell[];
  strip: { buttons: string[]; count: string } | null;
  zone: DropZone;
}

export interface LootData {
  live: boolean;
  rows: { serial: number; name: string; state: string }[];
  none: string | null;
  open: string;
  loot: string;
  loot_all: string | null;
}

export interface GoodRow {
  serial: number;
  picture: string | null;
  name: string;
  left: string;
  price: string;
  count: Colored;
  hover: TipKey;
}

export interface ShopData {
  live: boolean;
  goods: GoodRow[];
  total: string;
  gold: string | null;
  deal: string;
  clear: string;
  close: string;
  step_down: string;
  step_up: string;
}

export interface TradedItem {
  serial: number;
  picture: string | null;
  amount: string | null;
  hover: TipKey;
}

export interface TradeData {
  live: boolean;
  sides: { head: Colored; mine: boolean; items: TradedItem[] }[];
  coins: { label: string; platinum: boolean; words: string; owned: string }[];
  theirs: { label: string; value: string }[];
  accept: Colored | null;
  cancel: string | null;
  zone: DropZone;
}

export interface OldMenuData {
  live: boolean;
  entries: { picture: string | null; name: string }[];
  cancel: string | null;
}

export interface BookData {
  writing: boolean;
  by: string | null;
  title: string;
  author: string;
  title_hint: string;
  author_hint: string;
  pages: { number: string; words: string; caret: number | null }[];
  lines: number;
  /** Counts the typing on the pages: each draws the kept words again. */
  edits: number;
  turns: string[];
  save: string | null;
  close: string | null;
  paper: string;
  ink: string;
}

export interface BoardData {
  live: boolean;
  posts: { serial: number; subject: string; poster: string; indent: number; reading: boolean }[];
  text: string;
  subject: string;
  body: string;
  subject_hint: string;
  text_hint: string;
  post: string;
  reply: string | null;
  remove: string | null;
  close: string;
  paper: string;
  ink: string;
}

export interface PaperdollData {
  live: boolean;
  figure: string | null;
  out_of_sight: string | null;
  health: number | null;
  rows: { serial: number; picture: string | null; words: string; hover: TipKey }[];
  nothing: string | null;
  dresses: boolean;
  buttons: string[];
  close: string | null;
  zone: DropZone | null;
}

export interface EntryData {
  live: boolean;
  description: string;
  hint: string;
  words: string;
  focus: boolean;
  okay: string | null;
  cancel: string | null;
  take_control: string | null;
}

export interface RaceData {
  live: boolean;
  styles: { label: string; choices: string[]; chosen: number }[];
  figure: string | null;
  paints: { label: string; color: string; picking: boolean }[];
  hint: string;
  palette: { columns: number; hues: string[]; chosen: number } | null;
  change: string | null;
  keep: string | null;
}

export interface TipData {
  words: string;
  previous: string | null;
  next: string | null;
}

/** A grid of hues with the shade slider under it. */
export interface HueGridData {
  columns: number;
  cells: string[];
  chosen: number;
  shade: number;
  shade_least: number;
  shade_most: number;
  shade_words: string;
}

export interface DyeData {
  live: boolean;
  grid: HueGridData;
  tub: string | null;
  hue: string;
  okay: string | null;
  eyedropper: Colored | null;
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
  grids: Framed<GridData>[];
  loot: Framed<LootData> | null;
  split: Framed<SplitData> | null;
  shop: Framed<ShopData> | null;
  trades: Framed<TradeData>[];
  old_menu: Framed<OldMenuData> | null;
  book: Framed<BookData> | null;
  board: Framed<BoardData> | null;
  paperdoll: Framed<PaperdollData> | null;
  entry: Framed<EntryData> | null;
  race: Framed<RaceData> | null;
  tip: Framed<TipData> | null;
  dye: Framed<DyeData> | null;
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
