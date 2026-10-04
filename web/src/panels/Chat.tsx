import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { Note } from './Note';
import type { ChannelsData, ChatPanelData, Send } from './types';
import { WishField } from './WishField';

const ENTER = 'Enter';

/** The lines said in the channel, the newest at the bottom: the view follows the newest while it shows. */
function Lines({ lines }: { lines: ChannelsData['lines'] }) {
  const box = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  useLayoutEffect(() => {
    const shown = box.current;
    if (shown && atEnd.current) shown.scrollTop = shown.scrollHeight;
  }, [lines]);
  return (
    <div
      class="chat-lines"
      ref={box}
      onScroll={(event) => {
        const shown = event.currentTarget;
        atEnd.current = shown.scrollTop + shown.clientHeight >= shown.scrollHeight;
      }}
    >
      {lines.map((line, at) => (
        <p class="chat-said" key={at}>
          <span class="goal-words">{line.who} </span>
          <span>{line.words}</span>
        </p>
      ))}
    </div>
  );
}

/** The channels, their buttons, the small box, the lines and the fields to talk in. */
function Channels({ data, live, send }: { data: ChannelsData; live: boolean; send: Send }) {
  const [words, setWords] = useState('');
  /** The field of the small box, which hides its words for a password. */
  const asked = {
    class: 'field',
    value: data.asking?.words ?? '',
    onInput: (event: { currentTarget: HTMLInputElement }) => send({ asking: event.currentTarget.value }),
    onKeyDown: (event: KeyboardEvent) => {
      if (event.key === ENTER) send({ answer: true });
    },
  };
  return (
    <>
      <div class="chat-channels" title={data.row_hint}>
        {data.rows.map((row) => (
          <button
            type="button"
            class={`button row chat-channel${row.picked ? ' chosen' : ''}`}
            key={row.name}
            disabled={!live}
            onClick={() => send({ pick: row.name })}
            onDblClick={() => send({ join: row.name })}
          >
            <span class={row.here ? 'goal-words' : undefined}>{row.name}</span>
            {row.locked && <span class="small waiting">{row.locked}</span>}
          </button>
        ))}
      </div>
      {data.buttons.length > 0 && (
        <div class="button-row">
          {data.buttons.map((words, at) => (
            <button type="button" class={`button${at === 0 ? ' goal' : ''}`} key={words} onClick={() => send({ button: at })}>
              {words}
            </button>
          ))}
        </div>
      )}
      {data.asking && (
        <div class="button-row">
          <span class="dim">{data.asking.label}</span>
          {data.asking.hides ? <input type="password" {...asked} /> : <input type="text" {...asked} />}
          <button type="button" class="button goal" onClick={() => send({ answer: true })}>
            {data.asking.okay}
          </button>
          <button type="button" class="button dim" onClick={() => send({ answer: false })}>
            {data.asking.cancel}
          </button>
        </div>
      )}
      <Lines lines={data.lines} />
      {live && (
        <>
          <input
            class="field"
            placeholder={data.say_hint}
            value={words}
            onInput={(event) => setWords(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key !== ENTER) return;
              send({ say: words });
              setWords('');
            }}
          />
          <WishField hint={data.wish_hint} words={data.find} send={(wish) => send({ wish })} />
        </>
      )}
    </>
  );
}

/**
 * The chat of the shard: its channels and the lines said in them; the box
 * for the chat name the shard asks for; or the button that turns the chat
 * on. Lines are shard words, shown as text.
 */
export function Chat({ data, send }: { data: ChatPanelData; send: Send }) {
  const [name, setName] = useState('');
  return (
    <div class="chat-panel">
      {data.channels && <Channels data={data.channels} live={data.live} send={send} />}
      {data.name_box && (
        <>
          <p class="dim">{data.name_box.words}</p>
          {data.live && (
            <div class="button-row">
              <input class="field" maxLength={data.name_box.most} value={name} onInput={(event) => setName(event.currentTarget.value)} />
              <button type="button" class="button goal" onClick={() => send({ name })}>
                {data.name_box.okay}
              </button>
            </div>
          )}
        </>
      )}
      {data.turn_on && (
        <button type="button" class="button goal" onClick={() => send({ turn_on: true })}>
          {data.turn_on}
        </button>
      )}
      <Note note={data.note} />
    </div>
  );
}
