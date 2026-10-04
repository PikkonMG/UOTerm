import type { RefObject } from 'preact';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { isField } from '../input/keys';
import { ArtFeed, pixelsOf, type FeedView } from '../net/art';
import type {
  Card,
  ColorGrid,
  CreationModel,
  CreationWords,
  LookPage,
  NamePage,
  PointsGroup,
  ProfessionPage,
  ShownPicture,
  TownPage,
  TradePage,
} from './creation_model';

interface CreationProps {
  model: CreationModel;
  words: CreationWords;
  /** The character is made: it goes to the shard. */
  onFinish(): void;
  /** Back from the first page: the character list again. */
  onLeave(): void;
}

/** The feet of the figure stand this far above the bottom of its box, as in the window. */
const FLOOR_PAD = 26;
/** The shadow under the feet of the figure. */
const SHADOW_RADIUS_X = 34;
const SHADOW_RADIUS_Y = 8;
const SHADOW_COLOR = '--text-shadow';
const HALF = 2;
const FULL_TURN = Math.PI * 2;
/** The side of the picture of a profession on its card. */
const CARD_ICON = 56;
/** The side of the map of a start town. */
const TOWN_MAP_SIDE = 300;
const MINUS = '-';
const PLUS = '+';
const KEY_NEXT = 'Enter';
const KEY_BACK = 'Escape';
const KEY_LEFT = 'ArrowLeft';
const KEY_RIGHT = 'ArrowRight';

const rgb = ([red, green, blue]: [number, number, number]) => `rgb(${red}, ${green}, ${blue})`;

/** A field that takes the keys once it shows, as the one field of a page does in the window. */
function useFocused(): RefObject<HTMLInputElement | null> {
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => field.current?.focus(), []);
  return field;
}

/** The feed view of `model` that redraws the screen whenever something it fetched comes. */
function redrawing(model: CreationModel, redraw: () => void): FeedView {
  const then =
    <A extends unknown[]>(call: (...args: A) => void) =>
    (...args: A) => {
      call(...args);
      redraw();
    };
  return {
    artWanted: () => model.artWanted(),
    artArrived: then((key, width, height, x, y) => model.artArrived(key, width, height, x, y)),
    artMissing: then((key) => model.artMissing(key)),
    artForgotten: () => model.artForgotten(),
    dataWanted: () => model.dataWanted(),
    dataArrived: then((path, json) => model.dataArrived(path, json)),
    dataMissing: then((path) => model.dataMissing(path)),
    postsWanted: () => model.postsWanted(),
    postArrived: then((key, json) => model.postArrived(key, json)),
    postMissing: then((key) => model.postMissing(key)),
  };
}

/** A canvas of the device pixels of `width` by `height` CSS pixels, with its 2D context. */
function sized(canvas: HTMLCanvasElement, width: number, height: number): CanvasRenderingContext2D | null {
  const ratio = window.devicePixelRatio;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  const context = canvas.getContext('2d');
  if (!context) return null;
  context.scale(ratio, ratio);
  context.imageSmoothingEnabled = false;
  return context;
}

/**
 * The making of a new character: the steps at the top, the figure at the
 * left, the page in the middle, and Back and Next at the bottom with the
 * reason Next waits. Enter is Next, Esc is Back, and the arrows turn the
 * figure. The view runs every rule; this draws its screen.
 */
export function Creation({ model, words, onFinish, onLeave }: CreationProps) {
  const [, setDrawn] = useState(0);
  const redraw = () => setDrawn((count) => count + 1);
  const feed = useRef<ArtFeed | null>(null);
  const [pickerRow, setPickerRow] = useState<number | null>(null);

  useEffect(() => {
    const started = new ArtFeed(redrawing(model, redraw));
    feed.current = started;
    return () => started.close();
  }, [model]);
  // Each drawing names what it shows; the feed fetches what the page lacks.
  useEffect(() => feed.current?.pump());

  const change = (call: () => void) => {
    call();
    redraw();
  };
  const goNext = () => {
    if (model.next()) onFinish();
    else redraw();
  };
  const goBack = () => {
    if (model.back()) onLeave();
    else redraw();
  };

  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (pickerRow !== null) {
        if (event.key === KEY_BACK) setPickerRow(null);
        return;
      }
      const keys: Record<string, () => void> = {
        [KEY_NEXT]: goNext,
        [KEY_BACK]: goBack,
        ...(isField(event.target) ? {} : { [KEY_LEFT]: () => change(() => model.turn(false)), [KEY_RIGHT]: () => change(() => model.turn(true)) }),
      };
      const act = keys[event.key];
      if (!act) return;
      event.preventDefault();
      act();
    };
    window.addEventListener('keydown', down);
    return () => window.removeEventListener('keydown', down);
  });

  const screen = model.screen();
  const page = screen.page;
  return (
    <section class="panel creation">
      <header class="creation-header">
        <h1 class="title">{words.title}</h1>
        <ol class="stages">
          {screen.stages.map((stage, at) => (
            <li key={stage.words} class={`stage ${stage.progress.toLowerCase()}`}>
              <span class="stage-dot">{at + 1}</span>
              {stage.words}
            </li>
          ))}
        </ol>
      </header>
      <div class="creation-middle">
        <aside class="preview">
          <Figure model={model} figure={screen.preview.figure} noArt={words.no_art} />
          <div class="button-row">
            <button type="button" class="button" onClick={() => change(() => model.turn(false))}>
              {words.turn_left}
            </button>
            <button type="button" class="button" onClick={() => change(() => model.turn(true))}>
              {words.turn_right}
            </button>
          </div>
          <p class="heading">{screen.preview.title}</p>
          <p class="dim">{screen.preview.about}</p>
        </aside>
        <div class="creation-page">
          {!screen.ready && <p class="waiting">{words.loading}</p>}
          {screen.ready && page.step === 'Look' && <Look page={page} model={model} words={words} change={change} />}
          {screen.ready && page.step === 'Profession' && <Professions page={page} model={model} words={words} change={change} />}
          {screen.ready && page.step === 'Trade' && (
            <Trade page={page} model={model} words={words} change={change} pickerRow={pickerRow} setPickerRow={setPickerRow} />
          )}
          {screen.ready && page.step === 'Town' && <Towns page={page} model={model} words={words} change={change} />}
          {screen.ready && page.step === 'Name' && <Name page={page} model={model} words={words} change={change} />}
        </div>
      </div>
      <footer class="creation-footer">
        <p class={screen.footer.blocked ? 'waiting' : 'faint'}>{screen.footer.words}</p>
        <button type="button" class="button big" onClick={goBack}>
          {words.back}
        </button>
        <button type="button" class="button big goal" disabled={screen.footer.blocked} onClick={goNext}>
          {screen.footer.next}
        </button>
      </footer>
    </section>
  );
}

interface PageProps<P> {
  page: P;
  model: CreationModel;
  words: CreationWords;
  change(call: () => void): void;
}

/** The figure of the new character standing in its box, its feet on a shadow, at the whole scale that fits. */
function Figure({ model, figure, noArt }: { model: CreationModel; figure: ShownPicture | null; noArt: string }) {
  const box = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [, setSize] = useState(0);

  useEffect(() => {
    const resized = () => setSize((count) => count + 1);
    window.addEventListener('resize', resized);
    return () => window.removeEventListener('resize', resized);
  }, []);

  useLayoutEffect(() => {
    const pixels = figure ? pixelsOf(figure.key) : undefined;
    if (!box.current || !canvas.current || !figure || !pixels) return;
    const width = box.current.clientWidth;
    const height = box.current.clientHeight;
    const context = sized(canvas.current, width, height);
    if (!context) return;
    const scale = model.previewScale(width, height - FLOOR_PAD * HALF, figure.width, figure.height);
    const shown = { width: figure.width * scale, height: figure.height * scale };
    const floor = height - FLOOR_PAD;
    const left = Math.min(Math.max(width / HALF - figure.anchor_x * scale, 0), width - shown.width);
    context.fillStyle = getComputedStyle(document.documentElement).getPropertyValue(SHADOW_COLOR);
    context.beginPath();
    context.ellipse(width / HALF, floor, SHADOW_RADIUS_X, SHADOW_RADIUS_Y, 0, 0, FULL_TURN);
    context.fill();
    context.drawImage(pixels, Math.round(left), Math.round(floor - shown.height), shown.width, shown.height);
  });

  return (
    <div class="figure-box" ref={box}>
      {figure ? <canvas ref={canvas} /> : <p class="faint">{noArt}</p>}
    </div>
  );
}

/** A picture of the art fitted into a square of `side`, keeping its shape. */
function PictureBox({ picture, side, faint }: { picture: ShownPicture | null; side: number; faint: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useLayoutEffect(() => {
    const pixels = picture ? pixelsOf(picture.key) : undefined;
    if (!canvas.current || !picture || !pixels) return;
    const context = sized(canvas.current, side, side);
    if (!context) return;
    const scale = Math.min(side / picture.width, side / picture.height);
    const width = picture.width * scale;
    const height = picture.height * scale;
    context.drawImage(pixels, (side - width) / HALF, (side - height) / HALF, width, height);
  });
  return <canvas ref={canvas} class={`card-icon${faint ? ' locked' : ''}`} width={side} height={side} />;
}

function Look({ page, model, words, change }: PageProps<LookPage>) {
  const styles = (label: string, chips: LookPage['hair'], pick: (at: number) => void) => (
    <div class="style-group">
      <span class="label">{label}</span>
      <div class="chips">
        {chips.map((chip, at) => (
          <button type="button" key={chip.words} class={`button chip${chip.picked ? ' chosen' : ''}`} onClick={() => change(() => pick(at))}>
            {chip.words}
          </button>
        ))}
      </div>
    </div>
  );
  return (
    <>
      <h2 class="heading">{words.body}</h2>
      <div class="chips">
        {[false, true].map((female) => (
          <button
            type="button"
            key={String(female)}
            class={`button chip${page.female === female ? ' chosen' : ''}`}
            onClick={() => change(() => model.setFemale(female))}
          >
            {female ? words.female : words.male}
          </button>
        ))}
        {page.races.map((race) => (
          <button
            type="button"
            key={race.race}
            class={`button chip${race.picked ? ' chosen' : ''}${race.locked ? ' faint' : ''}`}
            onClick={() => change(() => model.setRace(race.race))}
          >
            {race.words}
          </button>
        ))}
      </div>
      <div class="styles">
        {styles(words.hair, page.hair, (at) => model.setHair(at))}
        {page.beards && styles(words.beard, page.beards, (at) => model.setBeard(at))}
      </div>
      <div class="color-grids">
        {page.colors.map((grid) => (
          <Palette key={grid.paint} grid={grid} pick={(at) => change(() => model.setColor(grid.paint, at))} />
        ))}
      </div>
    </>
  );
}

/** The hues of one color as a grid of boxes in their colors, the picked one ringed. */
function Palette({ grid, pick }: { grid: ColorGrid; pick(at: number): void }) {
  return (
    <div class="palette">
      <span class="label">{grid.words}</span>
      <div class="hue-grid" style={{ gridTemplateColumns: `repeat(${grid.columns}, 1fr)` }}>
        {grid.cells.map((cell, at) => (
          <button
            type="button"
            key={at}
            aria-label={`${grid.words} ${at + 1}`}
            class={`hue${at === grid.picked ? ' picked' : ''}${cell ? '' : ' unknown'}`}
            style={cell ? { background: rgb(cell) } : undefined}
            onClick={() => pick(at)}
          />
        ))}
      </div>
    </div>
  );
}

function Professions({ page, model, words, change }: PageProps<ProfessionPage>) {
  return (
    <>
      <h2 class="heading">{words.profession}</h2>
      <p class="dim">{words.profession_hint}</p>
      <div class="cards">
        {page.cards.map((card) => (
          <ProfessionCard key={card.true_name} card={card} pick={() => change(() => model.pickProfession(card.true_name))} />
        ))}
      </div>
      {page.details.map((line) => (
        <p key={line}>{line}</p>
      ))}
    </>
  );
}

function ProfessionCard({ card, pick }: { card: Card; pick(): void }) {
  return (
    <button
      type="button"
      class={`card${card.picked ? ' chosen' : ''}${card.locked ? ' faint' : ''}`}
      title={card.about}
      disabled={card.locked}
      onClick={pick}
    >
      <PictureBox picture={card.picture} side={CARD_ICON} faint={card.locked} />
      <span class="card-words">
        <span class="heading">{card.name}</span>
        <span class={card.locked ? 'waiting' : 'dim'}>{card.short}</span>
      </span>
    </button>
  );
}

/** A number the player sets: a minus button, a bar, a plus button, and the number. */
function Points({ value, least, most, label, set }: { value: number; least: number; most: number; label: string; set(value: number): void }) {
  return (
    <div class="points">
      <button type="button" class="button" onClick={() => set(value - 1)}>
        {MINUS}
      </button>
      <input
        type="range"
        aria-label={label}
        min={least}
        max={most}
        value={value}
        onInput={(event) => set(Number(event.currentTarget.value))}
      />
      <button type="button" class="button" onClick={() => set(value + 1)}>
        {PLUS}
      </button>
      <span class="number">{value}</span>
    </div>
  );
}

/** A heading with the total at its right: in the goal color when the values add up to it. */
function TotalHeading({ words, group }: { words: string; group: PointsGroup }) {
  return (
    <div class="total-heading">
      <h2 class="heading">{words}</h2>
      <span class={group.whole ? 'goal-words' : 'waiting'}>{group.total}</span>
    </div>
  );
}

function Rules({ group }: { group: PointsGroup }) {
  return (
    <>
      <p class="dim">{group.rule}</p>
      {group.limit && <p class="waiting">{group.limit}</p>}
    </>
  );
}

interface TradeProps extends PageProps<TradePage> {
  pickerRow: number | null;
  setPickerRow(row: number | null): void;
}

function Trade({ page, model, words, change, pickerRow, setPickerRow }: TradeProps) {
  return (
    <div class="trade">
      <div class="trade-part">
        <TotalHeading words={words.stats} group={page.stats} />
        {page.stats.rows.map((row, at) => (
          <div class="points-row" key={row.words}>
            <span>{row.words}</span>
            <Points value={row.value} least={page.stats.least} most={page.stats.most} label={row.words} set={(value) => change(() => model.setStat(at, value))} />
          </div>
        ))}
        <Rules group={page.stats} />
      </div>
      <div class="trade-part">
        <TotalHeading words={words.skills} group={page.skills} />
        {page.skills.rows.map((row, at) => (
          <div class="points-row" key={at}>
            <div class="skill-picker">
              <button type="button" class={`button${row.picked ? '' : ' waiting'}`} onClick={() => setPickerRow(pickerRow === at ? null : at)}>
                {row.words}
              </button>
              {pickerRow === at && (
                <SkillList
                  model={model}
                  row={at}
                  search={words.search}
                  pick={(skill) => {
                    setPickerRow(null);
                    change(() => model.setSkill(at, skill));
                  }}
                />
              )}
            </div>
            <Points value={row.value} least={page.skills.least} most={page.skills.most} label={row.words} set={(value) => change(() => model.setSkillValue(at, value))} />
          </div>
        ))}
        <Rules group={page.skills} />
      </div>
    </div>
  );
}

/** The skills a row picks from, with a search field. A skill of another row shows but cannot be picked. */
function SkillList({ model, row, search, pick }: { model: CreationModel; row: number; search: string; pick(skill: number): void }) {
  const [words, setWords] = useState('');
  const field = useFocused();
  return (
    <div class="popup">
      <input ref={field} class="field" placeholder={search} value={words} onInput={(event) => setWords(event.currentTarget.value)} />
      <div class="popup-list">
        {model.skillChoices(row, words).map((choice) => (
          <button
            type="button"
            key={choice.skill}
            class={`button row${choice.picked ? ' chosen' : ''}`}
            disabled={choice.taken}
            onClick={() => pick(choice.skill)}
          >
            {choice.words}
          </button>
        ))}
      </div>
    </div>
  );
}

function Towns({ page, model, words, change }: PageProps<TownPage>) {
  const [mapFailed, setMapFailed] = useState<string | null>(null);
  const picked = page.towns.find((town) => town.picked);
  const map = page.map && page.map.path !== mapFailed ? page.map : null;
  const scale = map ? TOWN_MAP_SIDE / map.tiles : 0;
  const pictureSide = map ? map.span * scale : 0;
  return (
    <>
      <h2 class="heading">{words.town}</h2>
      <div class="towns">
        <div class="town-list">
          {page.towns.map((town, at) => (
            <button type="button" key={`${at}-${town.name}`} class={`town${town.picked ? ' chosen' : ''}`} onClick={() => change(() => model.setTown(at))}>
              <span class={town.picked ? 'goal-words' : ''}>{town.name}</span>
              {town.facet && <span class="faint">{town.facet}</span>}
              <span class="dim">{town.building}</span>
            </button>
          ))}
        </div>
        <div class="town-about">
          {map && (
            <div class="town-map" style={{ width: `${TOWN_MAP_SIDE}px`, height: `${TOWN_MAP_SIDE}px` }}>
              <img
                src={map.path}
                alt=""
                style={{
                  width: `${pictureSide}px`,
                  height: `${pictureSide}px`,
                  left: `${(TOWN_MAP_SIDE - pictureSide) / HALF}px`,
                  top: `${(TOWN_MAP_SIDE - pictureSide) / HALF}px`,
                }}
                onError={() => setMapFailed(map.path)}
              />
              <span class="pin" />
              {picked && <span class="pin-name">{picked.name}</span>}
            </div>
          )}
          <p class="town-words">{page.words}</p>
        </div>
      </div>
    </>
  );
}

function Name({ page, model, words, change }: PageProps<NamePage>) {
  const field = useFocused();
  return (
    <div class="name-page">
      <div class="name-part">
        <h2 class="heading">{words.name}</h2>
        <input
          ref={field}
          class={`field name-field${page.good ? ' good' : ''}`}
          placeholder={words.name_hint}
          value={page.name}
          onInput={(event) => {
            const typed = event.currentTarget.value;
            change(() => model.setName(typed));
          }}
        />
        <p class={page.good ? 'goal-words' : 'waiting'}>{page.verdict}</p>
        <ul class="rules">
          {page.rules.map((rule) => (
            <li key={rule} class="dim">
              {rule}
            </li>
          ))}
        </ul>
      </div>
      <div class="summary">
        <h2 class="heading">{words.summary}</h2>
        {page.summary.map((row) => (
          <div class="summary-row" key={row.label}>
            <span class="label">{row.label}</span>
            {row.label === words.colors ? (
              <span class="swatches">
                {page.swatches.map((swatch) => (
                  <span key={swatch.words} class="swatch" title={swatch.words} style={swatch.color ? { background: rgb(swatch.color) } : undefined} />
                ))}
              </span>
            ) : (
              <span>{row.value}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
