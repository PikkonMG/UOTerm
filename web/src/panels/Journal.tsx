import { useState } from 'preact/hooks';
import { wheelNotches } from '../input/pointer';
import type { JournalData, Send } from './types';

const ENTER = 'Enter';
const ESCAPE = 'Escape';

/** The field of a name, which sends it on Enter and gives up on Escape. */
function NameField({ hint, done }: { hint: string; done(name: string | null): void }) {
  const [name, setName] = useState('');
  return (
    <input
      class="field small"
      placeholder={hint}
      value={name}
      ref={(field) => field?.focus()}
      onInput={(event) => setName(event.currentTarget.value)}
      onKeyDown={(event) => {
        if (event.key === ENTER) done(name);
        if (event.key === ESCAPE) done(null);
      }}
      onBlur={() => done(null)}
    />
  );
}

/**
 * The journal: its tabs, with "+" to add one and a menu on a right click
 * to rename it, flip the kinds of lines it shows or delete it; the search,
 * Save, the filters, and the lines, newest at the foot. The wheel reads
 * back.
 */
export function Journal({ data, send }: { data: JournalData; send: Send }) {
  const [adding, setAdding] = useState(false);
  const [menu, setMenu] = useState<number | null>(null);
  const [renaming, setRenaming] = useState(false);
  const tab = menu === null ? undefined : data.tabs[menu];
  return (
    <div class="journal">
      <div class="journal-tabs">
        {data.tabs.map((each, at) => (
          <button
            type="button"
            key={at}
            title={data.tab_hint}
            class={`button segment${each.chosen ? ' chosen' : ''}`}
            onClick={() => send({ tab: at })}
            onContextMenu={(event) => {
              event.preventDefault();
              setMenu(menu === at ? null : at);
              setRenaming(false);
            }}
          >
            {each.name}
          </button>
        ))}
        {adding ? (
          <NameField
            hint={data.tab_name_hint}
            done={(name) => {
              setAdding(false);
              if (name) send({ new_tab: name });
            }}
          />
        ) : (
          <button type="button" class="button goal-words" title={data.new_tab_hint} onClick={() => setAdding(true)}>
            {data.new_tab}
          </button>
        )}
      </div>
      {tab && menu !== null && (
        <div class="popup menu journal-menu">
          {renaming ? (
            <NameField
              hint={data.tab_name_hint}
              done={(name) => {
                setRenaming(false);
                if (name) send({ rename: { tab: menu, name } });
              }}
            />
          ) : (
            <button type="button" class="button row" onClick={() => setRenaming(true)}>
              {data.rename}
            </button>
          )}
          {data.kinds.map((words, kind) => (
            <label class="check" key={kind}>
              <input type="checkbox" checked={tab.kinds[kind]} onClick={() => send({ kind: { tab: menu, kind } })} />
              {words}
            </label>
          ))}
          <button
            type="button"
            class="button row alarm"
            onClick={() => {
              setMenu(null);
              send({ delete_tab: menu });
            }}
          >
            {data.delete_tab}
          </button>
        </div>
      )}
      <div class="journal-tools">
        <input class="field small" placeholder={data.search_hint} value={data.search} onInput={(event) => send({ search: event.currentTarget.value })} />
        <button type="button" class="button" onClick={() => send({ save: true })}>
          {data.save}
        </button>
        {data.note && (
          <p class="journal-note small" style={{ color: data.note.color }}>
            {data.note.words}
          </p>
        )}
      </div>
      <div class="journal-filters">
        {data.filters.map((filter, at) => (
          <button type="button" key={at} class={`button segment small${filter.shown ? ' goal-words' : ' faint'}`} onClick={() => send({ filter: at })}>
            {filter.words}
          </button>
        ))}
      </div>
      <div
        class="journal-lines"
        onWheel={(event) => {
          event.preventDefault();
          send({ wheel: wheelNotches(event) });
        }}
      >
        {data.no_lines && <p class="faint">{data.no_lines}</p>}
        {data.lines.map((line, at) => (
          <p class="journal-line" key={at}>
            {line.stamp && <span class="faint">{line.stamp}</span>}
            {line.name && <span class="goal-words">{`${line.name}:`}</span>}
            <span style={{ color: line.color }}>{line.text}</span>
          </p>
        ))}
        {data.back && <p class="journal-back waiting shadowed small">{data.back}</p>}
      </div>
    </div>
  );
}
