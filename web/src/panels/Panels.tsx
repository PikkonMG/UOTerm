import { memo } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { InputEvent } from '../input/events';
import { Activity } from './Activity';
import { Board } from './Board';
import { Book } from './Book';
import { Build } from './Build';
import { Chat } from './Chat';
import { ChatLine } from './ChatLine';
import { ControlBar } from './ControlBar';
import { Dye } from './Dye';
import { Entry } from './Entry';
import { dropTarget, isCarrying, PANEL_ATTRIBUTE } from './drag';
import { Frame } from './Frame';
import { Grid } from './Grid';
import type { Hover } from './hover';
import { Hotbar } from './Hotbar';
import { Journal } from './Journal';
import { Launcher } from './Launcher';
import { Loot } from './Loot';
import { MapItem } from './MapItem';
import { Markers } from './Markers';
import { Near } from './Near';
import { OldMenu } from './OldMenu';
import { Pack } from './Pack';
import { Paperdoll } from './Paperdoll';
import { Picture } from './Picture';
import { Profile } from './Profile';
import { QuestArrow } from './QuestArrow';
import { Question } from './Question';
import { Race } from './Race';
import { Radar } from './Radar';
import { Report } from './Report';
import { Ring } from './Ring';
import { ShardGump } from './ShardGump';
import { Sheet } from './Sheet';
import { Shop } from './Shop';
import { Split } from './Split';
import { TargetBar } from './TargetBar';
import { Tip } from './Tip';
import { TitleBar } from './TitleBar';
import { Tooltip } from './Tooltip';
import { Trade } from './Trade';
import type { CarriedData, Framed, PanelAction, PanelData, Place, Point, TooltipData } from './types';
import { Vitals } from './Vitals';
import { WorldMap } from './WorldMap';
import './panels.css';

/** An area of the view the panels cover, as the view reads it. */
export interface CoveredArea {
  min: Point;
  max: Point;
}

interface PanelsProps {
  data: PanelData;
  /** Sends an action of the panel `panel` to the view. */
  send(panel: string, action: PanelAction): void;
  /** Gives the view an input event of a panel, as the words of the chat line. */
  input(event: InputEvent): void;
  /** Tells the view where the panels lie, in points of the view, when that changed. */
  covered(areas: CoveredArea[]): void;
}

const placeStyle = (place: Place) => ({ left: `${place.x}px`, top: `${place.y}px`, width: `${place.w}px`, height: `${place.h}px` });

/** A panel that draws again only when its data changed, not when another panel's did. */
const sameData = <P extends { data: unknown }>(before: P, after: P) => JSON.stringify(before.data) === JSON.stringify(after.data);
const QuietSheet = memo(Sheet, sameData);
const QuietJournal = memo(Journal, sameData);
const QuietGrid = memo(Grid, sameData);
const QuietWorldMap = memo(WorldMap, sameData);
const QuietGump = memo(ShardGump, (before, after) => before.scale === after.scale && sameData(before, after));

/**
 * The tooltip and the carried thing at the mouse: the one part of the
 * panels that follows the mouse, so a move draws only it.
 */
function AtMouse({ tooltip, carried }: { tooltip: TooltipData | null; carried: CarriedData | null }) {
  const [mouse, setMouse] = useState<Point>({ x: 0, y: 0 });
  useEffect(() => {
    const move = (event: PointerEvent) => setMouse({ x: event.clientX, y: event.clientY });
    window.addEventListener('pointermove', move);
    return () => window.removeEventListener('pointermove', move);
  }, []);
  return (
    <>
      {tooltip && !carried && <Tooltip data={tooltip} at={mouse} />}
      {carried && (
        <div class="carried" style={{ left: `${mouse.x}px`, top: `${mouse.y}px`, opacity: carried.alpha }}>
          {carried.picture ? <Picture picture={carried.picture} words={carried.words} /> : <span class="goal-words title-face">{carried.words}</span>}
        </div>
      )}
    </>
  );
}

/** The areas the panels cover now, from the page as it is drawn. */
function coveredAreas(root: HTMLElement): CoveredArea[] {
  return [...root.querySelectorAll(`[${PANEL_ATTRIBUTE}]`)].map((element) => {
    const box = element.getBoundingClientRect();
    return { min: { x: box.left, y: box.top }, max: { x: box.right, y: box.bottom } };
  });
}

/**
 * Every Modern panel over the world, each where the view lays it, in the
 * panel layer the UI scale grows; the words over the world; the tooltip
 * and the carried thing at the mouse. A thing carried by a drag lands
 * where the button comes up, on the zone of the panel under it.
 */
export function Panels({ data, send, input, covered }: PanelsProps) {
  const root = useRef<HTMLDivElement>(null);
  const lastCovered = useRef('');
  const carried = useRef(data.carried);
  carried.current = data.carried;
  const scale = data.look.ui_scale;
  const to = (panel: string) => (action: PanelAction) => send(panel, action);
  const hover: Hover = (tip) => send('tips', { over: tip });
  const frameOf = <T,>(panel: Framed<T>) => ({ ...panel.frame, hints: data.hints, scale, send: to(panel.frame.panel) });

  useEffect(() => {
    const up = (event: PointerEvent) => {
      if (!carried.current && !isCarrying()) return;
      const target = dropTarget(document.elementFromPoint(event.clientX, event.clientY));
      send('desk', { drop: { x: event.clientX, y: event.clientY, ...target, shift: event.shiftKey } });
    };
    window.addEventListener('pointerup', up);
    return () => window.removeEventListener('pointerup', up);
  }, []);

  // The page is measured when the view gave new panel data (its places,
  // its UI scale, or words that change a height), and not on other frames:
  // the game gives new data only when it changed.
  useLayoutEffect(() => {
    if (!root.current) return;
    const areas = coveredAreas(root.current);
    const json = JSON.stringify(areas);
    if (json === lastCovered.current) return;
    lastCovered.current = json;
    covered(areas);
  }, [data]);

  const chat = <ChatLine data={data.chat} send={to('chat')} input={input} />;
  const { journal, question } = data;
  return (
    <div class="panels-root" ref={root}>
      <TitleBar title={data.title} />
      <div class="vignette" />
      <div class="alarm-edge" style={{ opacity: data.alarm }} />
      <div class="panels" style={{ '--ui-scale': scale, '--panel-opacity': data.look.opacity }}>
        {data.waiting && (
          <section class="panel waiting-panel">
            <h1 class="title" style={{ color: data.waiting.heading.color }}>
              {data.waiting.heading.words}
            </h1>
            {data.waiting.lines.map((line, at) => (
              <p class="dim" key={at}>
                {line}
              </p>
            ))}
          </section>
        )}
        {data.bar && (
          <section class="panel control-bar" style={{ ...placeStyle(data.bar.place), height: undefined }} {...{ [PANEL_ATTRIBUTE]: 'bar' }}>
            <ControlBar data={data.bar} send={to('bar')} />
          </section>
        )}
        {data.activity && (
          <Frame {...frameOf(data.activity)}>
            <Activity data={data.activity.body} />
          </Frame>
        )}
        {data.vitals && (
          <Frame {...frameOf(data.vitals)}>
            <Vitals data={data.vitals.body} />
          </Frame>
        )}
        {data.pack && (
          <Frame {...frameOf(data.pack)}>
            <Pack data={data.pack.body} />
          </Frame>
        )}
        {data.near && (
          <Frame {...frameOf(data.near)}>
            <Near data={data.near.body} send={to('near')} hover={hover} />
          </Frame>
        )}
        {data.radar && (
          <Frame {...frameOf(data.radar)}>
            <Radar data={data.radar.body} send={to('radar')} />
          </Frame>
        )}
        {journal && (
          <Frame {...frameOf(journal)} foot={chat} glass={journal.body.glass} glassOpacity={journal.body.glass_opacity}>
            <QuietJournal data={journal.body} send={to('journal')} />
          </Frame>
        )}
        {data.chat.strip && !data.chat.hidden && (
          <section class="panel chat-strip" style={placeStyle(data.chat.strip)} {...{ [PANEL_ATTRIBUTE]: 'chat' }}>
            {chat}
          </section>
        )}
        {data.hotbar && (
          <Frame {...frameOf(data.hotbar)}>
            <Hotbar data={data.hotbar.body} picker={data.picker} send={to('hotbar')} hover={hover} />
          </Frame>
        )}
        {data.bars.map((bar) => (
          <Frame {...frameOf(bar)} key={bar.frame.panel}>
            <TargetBar data={bar.body} send={to(bar.frame.panel)} />
          </Frame>
        ))}
        {data.sheet && (
          <Frame {...frameOf(data.sheet)}>
            <QuietSheet data={data.sheet.body} send={to('sheet')} hover={hover} />
          </Frame>
        )}
        {data.grids.map((grid) => (
          <Frame {...frameOf(grid)} key={grid.frame.panel} glass={grid.body.glass} glassOpacity={grid.body.glass_opacity}>
            <QuietGrid data={grid.body} send={to(grid.frame.panel)} hover={hover} />
          </Frame>
        ))}
        {data.loot && (
          <Frame {...frameOf(data.loot)}>
            <Loot data={data.loot.body} send={to('loot')} />
          </Frame>
        )}
        {data.shop && (
          <Frame {...frameOf(data.shop)}>
            <Shop data={data.shop.body} send={to('shop')} hover={hover} />
          </Frame>
        )}
        {data.trades.map((trade) => (
          <Frame {...frameOf(trade)} key={trade.frame.panel}>
            <Trade data={trade.body} send={to(trade.frame.panel)} hover={hover} />
          </Frame>
        ))}
        {data.old_menu && (
          <Frame {...frameOf(data.old_menu)}>
            <OldMenu data={data.old_menu.body} send={to('old_menu')} />
          </Frame>
        )}
        {data.book && (
          <Frame {...frameOf(data.book)}>
            <Book data={data.book.body} send={to('book')} />
          </Frame>
        )}
        {data.board && (
          <Frame {...frameOf(data.board)}>
            <Board data={data.board.body} send={to('board')} />
          </Frame>
        )}
        {data.paperdoll && (
          <Frame {...frameOf(data.paperdoll)}>
            <Paperdoll data={data.paperdoll.body} send={to('paperdoll')} hover={hover} />
          </Frame>
        )}
        {data.race && (
          <Frame {...frameOf(data.race)}>
            <Race data={data.race.body} send={to('race')} />
          </Frame>
        )}
        {data.tip && (
          <Frame {...frameOf(data.tip)}>
            <Tip data={data.tip.body} send={to('tip')} />
          </Frame>
        )}
        {data.dye && (
          <Frame {...frameOf(data.dye)}>
            <Dye data={data.dye.body} send={to('dye')} />
          </Frame>
        )}
        {data.entry && (
          <Frame {...frameOf(data.entry)}>
            <Entry data={data.entry.body} send={to('entry')} />
          </Frame>
        )}
        {data.world_map && (
          <Frame {...frameOf(data.world_map)}>
            <QuietWorldMap data={data.world_map.body} send={to('world_map')} />
          </Frame>
        )}
        {data.markers && (
          <Frame {...frameOf(data.markers)}>
            <Markers data={data.markers.body} send={to('markers')} />
          </Frame>
        )}
        {data.map_items.map((map) => (
          <Frame {...frameOf(map)} key={map.frame.panel}>
            <MapItem data={map.body} send={to(map.frame.panel)} />
          </Frame>
        ))}
        {data.build && (
          <Frame {...frameOf(data.build)}>
            <Build data={data.build.body} send={to('build')} />
          </Frame>
        )}
        {data.channels && (
          <Frame {...frameOf(data.channels)}>
            <Chat data={data.channels.body} send={to('channels')} />
          </Frame>
        )}
        {data.profile && (
          <Frame {...frameOf(data.profile)}>
            <Profile data={data.profile.body} send={to('profile')} />
          </Frame>
        )}
        {data.gumps.map((gump) => (
          <QuietGump data={gump} scale={scale} send={to(gump.panel)} hover={hover} key={gump.panel} />
        ))}
        {data.quest_arrow && <QuestArrow data={data.quest_arrow} send={to('quest_arrow')} />}
        {data.launcher && (
          <Frame {...frameOf(data.launcher)}>
            <Launcher data={data.launcher.body} send={to('launcher')} />
          </Frame>
        )}
        {data.split && (
          <Frame {...frameOf(data.split)}>
            <Split data={data.split.body} send={to('split')} />
          </Frame>
        )}
        {question &&
          (question.framed ? (
            <Frame {...question.framed} hints={data.hints} scale={scale} send={to('question')}>
              <Question data={question} send={to('question')} />
            </Frame>
          ) : (
            <section class="panel guard-question" style={placeStyle(question.place)} {...{ [PANEL_ATTRIBUTE]: 'question' }}>
              <Question data={question} send={to('question')} />
            </section>
          ))}
        {data.report && <Report data={data.report} />}
        {data.ring && <Ring data={data.ring} send={to('ring')} />}
      </div>
      <AtMouse tooltip={data.tooltip} carried={data.carried} />
    </div>
  );
}
