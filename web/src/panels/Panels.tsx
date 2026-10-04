import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { InputEvent } from '../input/events';
import { Activity } from './Activity';
import { ChatLine } from './ChatLine';
import { ControlBar } from './ControlBar';
import { dropTarget, isCarrying, PANEL_ATTRIBUTE } from './drag';
import { Frame } from './Frame';
import type { Hover } from './hover';
import { Hotbar } from './Hotbar';
import { Journal } from './Journal';
import { Launcher } from './Launcher';
import { Near } from './Near';
import { Pack } from './Pack';
import { Picture } from './Picture';
import { Plates } from './Plates';
import { Question } from './Question';
import { Radar } from './Radar';
import { Report } from './Report';
import { Ring } from './Ring';
import { Sheet } from './Sheet';
import { Split } from './Split';
import { TargetBar } from './TargetBar';
import { TitleBar } from './TitleBar';
import { Tooltip } from './Tooltip';
import type { Framed, PanelAction, PanelData, Place, PlacedPlate, PlacedWords, Point } from './types';
import { Vitals } from './Vitals';
import './panels.css';

/** An area of the view the panels cover, as the view reads it. */
export interface CoveredArea {
  min: Point;
  max: Point;
}

interface PanelsProps {
  data: PanelData;
  plates: PlacedPlate[];
  floats: PlacedWords[];
  /** Sends an action of the panel `panel` to the view. */
  send(panel: string, action: PanelAction): void;
  /** Gives the view an input event of a panel, as the words of the chat line. */
  input(event: InputEvent): void;
  /** Tells the view where the panels lie, in points of the view, when that changed. */
  covered(areas: CoveredArea[]): void;
}

const placeStyle = (place: Place) => ({ left: `${place.x}px`, top: `${place.y}px`, width: `${place.w}px`, height: `${place.h}px` });

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
export function Panels({ data, plates, floats, send, input, covered }: PanelsProps) {
  const root = useRef<HTMLDivElement>(null);
  const lastCovered = useRef('');
  const carried = useRef(data.carried);
  const [mouse, setMouse] = useState<Point>({ x: 0, y: 0 });
  carried.current = data.carried;
  const scale = data.look.ui_scale;
  const to = (panel: string) => (action: PanelAction) => send(panel, action);
  const hover: Hover = (tip) => send('tips', { over: tip });
  const frameOf = <T,>(panel: Framed<T>) => ({ ...panel.frame, hints: data.hints, scale, send: to(panel.frame.panel) });

  useEffect(() => {
    const move = (event: PointerEvent) => setMouse({ x: event.clientX, y: event.clientY });
    const up = (event: PointerEvent) => {
      if (!carried.current && !isCarrying()) return;
      const target = dropTarget(document.elementFromPoint(event.clientX, event.clientY));
      send('desk', { drop: { x: event.clientX, y: event.clientY, ...target, shift: event.shiftKey } });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    return () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
  }, []);

  useLayoutEffect(() => {
    if (!root.current) return;
    const areas = coveredAreas(root.current);
    const json = JSON.stringify(areas);
    if (json === lastCovered.current) return;
    lastCovered.current = json;
    covered(areas);
  });

  const chat = <ChatLine data={data.chat} send={to('chat')} input={input} />;
  const { journal, question } = data;
  return (
    <div class="panels-root" ref={root}>
      <TitleBar title={data.title} />
      <Plates plates={plates} floats={floats} />
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
        {data.report && <Report data={data.report} />}
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
          <Frame {...frameOf(journal)} foot={chat}>
            <Journal data={journal.body} send={to('journal')} />
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
            <Sheet data={data.sheet.body} send={to('sheet')} hover={hover} />
          </Frame>
        )}
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
        {data.ring && <Ring data={data.ring} send={to('ring')} />}
      </div>
      {data.tooltip && !data.carried && <Tooltip data={data.tooltip} at={mouse} />}
      {data.carried && (
        <div class="carried" style={{ left: `${mouse.x}px`, top: `${mouse.y}px`, opacity: data.carried.alpha }}>
          {data.carried.picture ? <Picture picture={data.carried.picture} words={data.carried.words} /> : <span class="goal-words title-face">{data.carried.words}</span>}
        </div>
      )}
    </div>
  );
}
