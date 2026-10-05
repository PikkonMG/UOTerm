import { useState } from 'preact/hooks';
import { toPoints } from '../points';
import { followDrag, ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import { ShareBar } from './Rows';
import type { Choice, PartyData, Send, SheetData, SkillsData, SpellsData, StatRow, StatusRow, WornData } from './types';

const ENTER = 'Enter';
const DELETE = 'Delete';
/** The attribute of a group of skills a dragged skill moves into. */
const GROUP_ATTRIBUTE = 'data-group';
/** The locks of a skill or a stat: up, down, locked. */
const LOCK_UP = 0;
const LOCK_DOWN = 1;

interface PartProps {
  send: Send;
  hover?: Hover;
}

/** A row of buttons of which one is chosen. */
function Choices({ choices, pick }: { choices: Choice[]; pick(at: number): void }) {
  return (
    <div class="sheet-choices">
      {choices.map((choice, at) => (
        <button type="button" key={at} class={`button segment${choice.chosen ? ' chosen' : ''}`} onClick={() => pick(at)}>
          {choice.words}
        </button>
      ))}
    </div>
  );
}

/** An arrow up, an arrow down, or a block for a locked skill or stat. */
function LockMark({ lock }: { lock: number }) {
  const shape = lock === LOCK_UP ? 'M8 3 L13 12 L3 12 Z' : lock === LOCK_DOWN ? 'M8 13 L3 4 L13 4 Z' : 'M4 4 H12 V12 H4 Z';
  return (
    <svg viewBox="0 0 16 16" class="lock-art" aria-hidden="true">
      <path d={shape} />
    </svg>
  );
}

/** A field and its button, which send the words on a click or on Enter. */
function WordsField({ hint, button, enabled = true, sent }: { hint: string; button: string; enabled?: boolean; sent(words: string): void }) {
  const [words, setWords] = useState('');
  const send = () => {
    if (!enabled || !words.trim()) return;
    sent(words);
    setWords('');
  };
  return (
    <div class="words-field">
      <input class="field small" placeholder={hint} value={words} onInput={(event) => setWords(event.currentTarget.value)} onKeyDown={(event) => event.key === ENTER && send()} />
      <button type="button" class={`button${enabled ? ' goal-words' : ' faint'}`} onClick={send}>
        {button}
      </button>
    </div>
  );
}

function Stat({ stat, send, hover }: PartProps & { stat: StatRow }) {
  return (
    <div class="fact" {...hoverOn(stat.hover, hover)}>
      <button type="button" class="lock" aria-label={stat.name} onClick={() => send({ stat_lock: stat.stat })}>
        <LockMark lock={stat.lock} />
      </button>
      <span class="dim small">{stat.name}</span>
      <span class="number">{stat.value}</span>
    </div>
  );
}

/** The worn view: the figure, the stats, the weight and the gold, the worn items and the field of what to wear. */
function Worn({ data, send, hover }: PartProps & { data: WornData }) {
  return (
    <div class="worn" {...{ [ZONE_ATTRIBUTE]: JSON.stringify(data.zone) }}>
      <div class="worn-top">
        <div class="doll">{data.doll ? <Picture picture={data.doll} words={data.name} /> : <span class="dim">{data.name}</span>}</div>
        <div class="facts">
          {data.stats.map((stat) => (
            <Stat stat={stat} send={send} hover={hover} key={stat.stat} />
          ))}
          {data.facts.map((fact, at) => (
            <div class="fact" key={at}>
              <span class="lock" />
              <span class="dim small">{fact.words}</span>
              <span class="number">{fact.value}</span>
            </div>
          ))}
        </div>
      </div>
      <p class="dim">{data.worn_words}</p>
      {data.nothing && <p class="faint small">{data.nothing}</p>}
      <div class="worn-rows">
        {data.rows.map((row) => (
          <div
            class="worn-row"
            key={row.serial}
            {...hoverOn(row.hover, hover)}
            onClick={() => send({ worn_click: row.serial })}
            onDblClick={() => send({ worn_double: row.serial })}
            onContextMenu={(event) => {
              event.preventDefault();
              send({ worn_menu: { serial: row.serial, ...toPoints(event.clientX, event.clientY) } });
            }}
            onPointerDown={(event) => {
              if (event.button === 0) followDrag(event, { carries: true, started: () => send({ worn_drag: row.serial }) });
            }}
          >
            <span class="cell-art">
              <Picture picture={row.picture} />
            </span>
            <span class="dim small worn-words">{row.words}</span>
            {row.wear && <ShareBar fill={row.wear.share} color={row.wear.color} />}
            <button
              type="button"
              class="button alarm small"
              onClick={(event) => {
                event.stopPropagation();
                send({ take_off: row.layer });
              }}
            >
              {row.take_off}
            </button>
          </div>
        ))}
      </div>
      {data.wear && (
        <>
          {data.wear.note && (
            <p class="small" style={{ color: data.wear.note.color }}>
              {data.wear.note.words}
            </p>
          )}
          <WordsField hint={data.wear.hint} button={data.wear.button} enabled={data.wear.on} sent={(wish) => send({ wear: wish })} />
        </>
      )}
    </div>
  );
}

/** The status view: the stats with their locks, and every fact of the status in its sections. */
function Status({ rows, send, hover }: PartProps & { rows: StatusRow[] }) {
  return (
    <div class="status-columns">
      {rows.map((row, at) => {
        switch (row.kind) {
          case 'title':
            return (
              <h3 class="heading goal-words small" key={at}>
                {row.words}
              </h3>
            );
          case 'stat':
            return <Stat stat={row} send={send} hover={hover} key={at} />;
          case 'fact':
            return (
              <div class="fact" key={at}>
                <span class="dim small">{row.words}</span>
                <span class="number">{row.value}</span>
              </div>
            );
        }
      })}
    </div>
  );
}

/** A group of skills: its fold mark, its name, which a second click renames, and its delete mark. */
function Group({ row, send, hover }: PartProps & { row: Extract<SkillsData['rows'][number], { kind: 'group' }> }) {
  const [selected, setSelected] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(row.name);
  return (
    <div class={`skill-group${selected ? ' chosen' : ''}`} {...{ [GROUP_ATTRIBUTE]: row.at }} {...hoverOn(row.hover, hover)}>
      <button type="button" class="button small" onClick={() => send({ fold_group: row.at })}>
        {row.fold}
      </button>
      {renaming ? (
        <input
          class="field small"
          value={name}
          ref={(field) => field?.focus()}
          onInput={(event) => setName(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key !== ENTER) return;
            send({ rename_group: { at: row.at, name } });
            setRenaming(false);
          }}
          onBlur={() => setRenaming(false)}
        />
      ) : (
        <button
          type="button"
          class="group-name goal-words"
          onClick={() => {
            if (selected) {
              setName(row.name);
              setRenaming(true);
            }
            setSelected(!selected);
          }}
          onKeyDown={(event) => event.key === DELETE && row.delete && send({ delete_group: row.at })}
        >
          {row.words}
        </button>
      )}
      {row.delete && (
        <button type="button" class="button alarm small" onClick={() => send({ delete_group: row.at })}>
          {row.delete}
        </button>
      )}
    </div>
  );
}

/** The skills: the sums, the buttons of the groups, the heads of the columns, and the groups or the table. */
function Skills({ data, send, hover }: PartProps & { data: SkillsData }) {
  return (
    <div class="skills">
      <div class="skills-bar">
        {data.reset_ask ? (
          <>
            <span class="waiting small">{data.reset_ask.words}</span>
            <button type="button" class="button alarm" onClick={() => send({ reset_answer: true })}>
              {data.reset_ask.yes}
            </button>
            <button type="button" class="button" onClick={() => send({ reset_answer: false })}>
              {data.reset_ask.no}
            </button>
          </>
        ) : (
          <>
            <span class="number dim small">{data.sums}</span>
            {data.new_group && (
              <button type="button" class="button goal-words" onClick={() => send({ new_group: true })}>
                {data.new_group}
              </button>
            )}
            {data.reset && (
              <button type="button" class="button" onClick={() => send({ reset_groups: true })}>
                {data.reset}
              </button>
            )}
          </>
        )}
      </div>
      <div class="skill-row skill-head">
        {data.columns.map((column, at) => (
          <button
            type="button"
            key={at}
            class={`column${column.chosen ? ' goal-words' : ' dim'}`}
            disabled={data.grouped}
            onClick={() => send({ sort: at })}
          >
            {column.words}
            {column.chosen && <LockMark lock={column.descending ? LOCK_DOWN : LOCK_UP} />}
          </button>
        ))}
      </div>
      <div class="skill-rows">
        {data.rows.map((row) =>
          row.kind === 'group' ? (
            <Group row={row} send={send} hover={hover} key={`group-${row.at}`} />
          ) : (
            <div
              class="skill-row"
              key={`skill-${row.id}`}
              {...hoverOn(row.hover, hover)}
              onPointerDown={(event) => {
                if (event.button !== 0) return;
                followDrag(event, {
                  carries: Boolean(row.buttons),
                  started: () => row.buttons && send({ drag_skill: row.id }),
                  dropped: (at) => {
                    const group = document.elementFromPoint(at.x, at.y)?.closest(`[${GROUP_ATTRIBUTE}]`)?.getAttribute(GROUP_ATTRIBUTE);
                    if (group) send({ move_skill: { skill: row.id, to: Number(group) } });
                  },
                });
              }}
            >
              <button type="button" class="lock" aria-label={row.name} onClick={() => send({ skill_lock: row.id })}>
                <LockMark lock={row.lock} />
              </button>
              <span class="skill-name">{row.name}</span>
              {row.values.map((value, at) => (
                <span class="number dim small" key={at}>
                  {value}
                </span>
              ))}
              {row.buttons && (
                <>
                  <button type="button" class="button small" onClick={() => send({ use_skill: row.id })}>
                    {row.buttons[0]}
                  </button>
                  <button type="button" class="button small" onClick={() => send({ pin_skill: row.id })}>
                    {row.buttons[1]}
                  </button>
                </>
              )}
            </div>
          ),
        )}
      </div>
    </div>
  );
}

/** The spells: the books, the spells of the chosen one, and what the chosen spell is. */
function Spells({ data, send, hover }: PartProps & { data: SpellsData }) {
  if (data.no_book) {
    return (
      <div class="spells">
        <p class="faint small">{data.no_book.words}</p>
        <div class="sheet-choices">
          {data.no_book.books.map(([name, title]) => (
            <button type="button" key={name} class="button goal-words" onClick={() => send({ open_book: name })}>
              {title}
            </button>
          ))}
        </div>
      </div>
    );
  }
  const { detail } = data;
  return (
    <div class="spells">
      <Choices choices={data.books} pick={(at) => send({ book: at })} />
      {data.empty && <p class="faint small">{data.empty}</p>}
      <div class="spell-parts">
        <div class="spell-list">
          {data.list.map((spell) => (
            <div
              key={spell.id}
              class={`spell-row${spell.chosen ? ' chosen' : ''}`}
              {...hoverOn(spell.hover, hover)}
              onClick={(event) => send({ spell: { id: spell.id, ctrl: event.ctrlKey, alt: event.altKey } })}
              onDblClick={() => send({ cast: spell.id })}
              onPointerDown={(event) => {
                if (event.button === 0) followDrag(event, { carries: true, started: () => send({ drag_spell: spell.id }) });
              }}
            >
              <span class="cell-art">
                <Picture picture={spell.icon} />
              </span>
              <span class="small">{spell.name}</span>
              {data.assign && <span class="assign waiting">{data.assign}</span>}
            </div>
          ))}
        </div>
        <div class="spell-detail">
          {data.pick && <p class="faint small">{data.pick}</p>}
          {detail && (
            <>
              <div class="spell-head">
                <span class="spell-icon">
                  <Picture picture={detail.icon} />
                </span>
                <div>
                  <p class="heading">{detail.name}</p>
                  {detail.group && <p class="dim small">{detail.group}</p>}
                </div>
              </div>
              {detail.power && <p class="goal-words title-face small">{detail.power}</p>}
              {detail.lines.map((line, at) => (
                <p class={`small${line.dim ? ' dim' : ''}`} key={at}>
                  {line.words}
                </p>
              ))}
              {detail.buttons && (
                <div class="button-row">
                  <button type="button" class="button goal-words" onClick={() => send({ cast: detail.id })}>
                    {detail.buttons[0]}
                  </button>
                  <button type="button" class="button" onClick={() => send({ pin_spell: detail.id })}>
                    {detail.buttons[1]}
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

/** The party: an invite, loot, leave and add, the ten places and the people near, and words to the party. */
function Party({ data, send, hover }: PartProps & { data: PartyData }) {
  return (
    <div class="party">
      {data.invite && (
        <div class="party-band">
          <span class="waiting small">{data.invite.words}</span>
          {data.invite.buttons && (
            <>
              <button type="button" class="button goal-words" onClick={() => send({ accept: true })}>
                {data.invite.buttons[0]}
              </button>
              <button type="button" class="button alarm" onClick={() => send({ decline: true })}>
                {data.invite.buttons[1]}
              </button>
            </>
          )}
        </div>
      )}
      <div class="party-band">
        {data.loot && (
          <button type="button" class="button" onClick={() => send({ loot: true })}>
            {data.loot}
          </button>
        )}
        {data.add && (
          <button type="button" class="button goal-words" onClick={() => send({ add: true })}>
            {data.add}
          </button>
        )}
        {data.leave && (
          <button type="button" class="button alarm" onClick={() => send({ leave: true })}>
            {data.leave}
          </button>
        )}
      </div>
      <div class="party-rows">
        {data.rows.map((row, at) => {
          switch (row.kind) {
            case 'place':
              return (
                <div class="party-row" key={at}>
                  <span class="number dim small">{row.number}</span>
                  {row.member ? (
                    <>
                      <button type="button" class={`member${row.member.chosen ? ' goal-words' : ''}`} {...hoverOn(row.member.hover, hover)} onClick={() => row.member && send({ member: row.member.serial })}>
                        <span>{row.member.name}</span>
                        <span class="pools">
                          {row.member.pools.map((share, pool) => (
                            <ShareBar fill={share} key={pool} />
                          ))}
                        </span>
                      </button>
                      {row.member.tell && (
                        <button type="button" class="button small" onClick={() => row.member && send({ tell_to: row.member.serial })}>
                          {row.member.tell}
                        </button>
                      )}
                      {row.member.kick && (
                        <button type="button" class="button alarm small" onClick={() => row.member && send({ kick: row.member.serial })}>
                          {row.member.kick}
                        </button>
                      )}
                    </>
                  ) : (
                    <span class="faint small">{row.empty}</span>
                  )}
                </div>
              );
            case 'near_title':
              return (
                <p class="dim small" key={at}>
                  {row.words}
                </p>
              );
            case 'near':
              return (
                <div class="party-row" key={at}>
                  <span class="member">{row.name}</span>
                  {row.invite && (
                    <button type="button" class="button goal-words small" onClick={() => send({ invite: row.serial })}>
                      {row.invite}
                    </button>
                  )}
                </div>
              );
          }
        })}
      </div>
      {data.tell && <WordsField hint={data.tell.hint} button={data.tell.say} sent={(words) => send({ say: words })} />}
    </div>
  );
}

/**
 * The sheet of the character: the Character tab with the worn and status
 * views and the buttons of the ability panels, the Skills, Spells and
 * Party tabs.
 */
export function Sheet({ data, send, hover }: { data: SheetData; send: Send; hover?: Hover }) {
  const { character } = data;
  return (
    <div class="sheet">
      <Choices choices={data.tabs} pick={(at) => send({ tab: at })} />
      {character && (
        <div class="sheet-tab">
          <div class="sheet-views">
            <Choices choices={character.views} pick={(at) => send({ view: at })} />
            <div class="sheet-panels">
              {character.panels.map((panel, at) => (
                <button type="button" key={at} class={`button segment${panel.chosen ? ' chosen' : ''}`} onClick={() => send({ panel: at })}>
                  {panel.words}
                </button>
              ))}
            </div>
          </div>
          {character.worn && <Worn data={character.worn} send={send} hover={hover} />}
          {character.status && <Status rows={character.status} send={send} hover={hover} />}
        </div>
      )}
      {data.skills && <Skills data={data.skills} send={send} hover={hover} />}
      {data.spells && <Spells data={data.spells} send={send} hover={hover} />}
      {data.party && <Party data={data.party} send={send} hover={hover} />}
    </div>
  );
}
