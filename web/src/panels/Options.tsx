import { useEffect, useState } from 'preact/hooks';
import { api } from '../net/api';
import { KeyCapture } from './KeyCapture';
import type { Control, HueData, KeysData, OptionsData, Send } from './types';

/** The names of the player fonts the server keeps. */
const FONTS_PATH = '/v1/fonts';

/** The props every control of a row gets: its row, and what it sends. */
interface RowProps {
  row: string;
  send: Send;
}

/** Sets a row to `value`. */
const setRow = (send: Send, row: string, value: unknown) => send({ set: { row, value } });

/** Sets a field of the entry `at` of a list row. */
const setField = (send: Send, row: string, at: number, name: string, value: unknown) => send({ field: { row, at, name, value } });

/**
 * Words the player types over many lines: they stay as he types them while
 * the field has the keys, and each change goes to the view, which reads
 * them; out of the field it shows what the view kept.
 */
function DraftArea({ words, hint, sent }: { words: string; hint: string; sent(words: string): void }) {
  const [draft, setDraft] = useState<string | null>(null);
  return (
    <textarea
      class="field option-lines"
      placeholder={hint}
      value={draft ?? words}
      onFocus={() => setDraft(words)}
      onBlur={() => setDraft(null)}
      onInput={(event) => {
        setDraft(event.currentTarget.value);
        sent(event.currentTarget.value);
      }}
    />
  );
}

/** A hue: its number in hex, and the swatch that opens the color picker. */
function HueBox({ hue, changed, swatch }: { hue: HueData; changed(words: string): void; swatch(): void }) {
  return (
    <span class="hue-box">
      <input class="field number hue-words" value={hue.words} onChange={(event) => changed(event.currentTarget.value)} />
      <button type="button" class="swatch" title={hue.hint} aria-label={hue.hint} style={{ background: hue.color }} onClick={swatch} />
    </span>
  );
}

/** The file of a row: a player font from the server's list, or a file the server reads. */
function FileBox({ row, send, words, hint, fonts }: RowProps & { words: string; hint: string; fonts: boolean }) {
  const [names, setNames] = useState<string[]>([]);
  useEffect(() => {
    if (!fonts) return;
    api<string[]>(FONTS_PATH).then(setNames, () => setNames([]));
  }, [fonts]);
  if (!fonts) return <input class="field" placeholder={hint} value={words} onChange={(event) => setRow(send, row, event.currentTarget.value)} />;
  return (
    <select class="field" value={words} onChange={(event) => setRow(send, row, event.currentTarget.value)}>
      <option value="">{hint}</option>
      {names.map((name) => (
        <option key={name} value={name}>
          {name}
        </option>
      ))}
    </select>
  );
}

/** Remove, beside an entry of a list. */
function Remove({ words, row, at, send }: RowProps & { words: string; at: number }) {
  return (
    <button type="button" class="button small" onClick={() => send({ remove: { row, at } })}>
      {words}
    </button>
  );
}

/** Add, under a list. */
function Add({ words, row, send }: RowProps & { words: string }) {
  return (
    <button type="button" class="button small" onClick={() => send({ add: row })}>
      {words}
    </button>
  );
}

/** The macros of the Macros page: each name, key, buttons and steps, and the default keys under them. */
function Keys({ keys, row, send, remove }: RowProps & { keys: KeysData; remove: string }) {
  const actionOptions = keys.groups.map((group) => (
    <optgroup label={group.label} key={group.label}>
      {group.actions.map((action) => (
        <option value={action.id} key={action.id}>
          {action.label}
        </option>
      ))}
    </optgroup>
  ));
  return (
    <div class="option-list">
      {keys.macros.map((binding, at) => (
        <div class="option-entry" key={at}>
          <div class="option-line">
            <input class="field" placeholder={keys.name_hint} value={binding.name} onChange={(event) => send({ macro_name: { at, words: event.currentTarget.value } })} />
            <KeyCapture words={binding.chord} at={at} pad={false} send={send} />
            <KeyCapture words={binding.pad} at={at} pad={true} send={send} />
            {binding.bound && (
              <button type="button" class="button small" onClick={() => send({ clear_keys: at })}>
                {keys.clear}
              </button>
            )}
            <button type="button" class={`button small${binding.open ? ' chosen' : ''}`} onClick={() => send({ steps: at })}>
              {keys.steps}
            </button>
            <Remove words={remove} row={row} at={at} send={send} />
          </div>
          {binding.open && (
            <div class="option-steps">
              {binding.steps.map((step, number) => (
                <div class="option-line" key={number}>
                  <span class="small dim number">{number + 1}</span>
                  <select class="field" value={step.action_id} onChange={(event) => send({ step_action: { at, step: number, words: event.currentTarget.value } })}>
                    {actionOptions}
                  </select>
                  {step.typed && (
                    <input
                      class="field"
                      placeholder={step.hint}
                      value={step.argument}
                      onChange={(event) => send({ step_argument: { at, step: number, words: event.currentTarget.value } })}
                    />
                  )}
                  {step.choices.length > 0 && (
                    <select class="field" value={step.typed ? '' : step.argument} onChange={(event) => send({ step_argument: { at, step: number, words: event.currentTarget.value } })}>
                      {step.typed && <option value="">{step.shown}</option>}
                      {step.choices.map((choice) => (
                        <option value={choice} key={choice}>
                          {choice}
                        </option>
                      ))}
                    </select>
                  )}
                  <button type="button" class="button small" onClick={() => send({ step_move: { at, step: number, up: true } })}>
                    {keys.up}
                  </button>
                  <button type="button" class="button small" onClick={() => send({ step_move: { at, step: number, up: false } })}>
                    {keys.down}
                  </button>
                  <button type="button" class="button small" onClick={() => send({ step_remove: { at, step: number } })}>
                    {remove}
                  </button>
                </div>
              ))}
              <select
                class="field"
                value=""
                onChange={(event) => {
                  if (event.currentTarget.value) send({ step_add: { at, action: event.currentTarget.value } });
                }}
              >
                <option value="">{keys.add_step}</option>
                {actionOptions}
              </select>
            </div>
          )}
        </div>
      ))}
      <Add words={keys.add} row={row} send={send} />
      {keys.defaults.map((list) => (
        <div class="option-defaults" key={list.title}>
          <p class="dim small">{list.title}</p>
          {list.lines.map((line) => (
            <p class="small" key={line}>
              {line}
            </p>
          ))}
        </div>
      ))}
    </div>
  );
}

/** The control of one row, by its kind. */
function RowControl({ control, row, send, remove }: RowProps & { control: Control; remove: string }) {
  const swatch = (at: number) => () => send({ swatch: { row, at } });
  switch (control.kind) {
    case 'toggle':
      return (
        <label class="option-line">
          <input type="checkbox" checked={control.on} onChange={(event) => setRow(send, row, event.currentTarget.checked)} />
          <span>{row}</span>
        </label>
      );
    case 'slider':
      return (
        <label class="option-line">
          <input
            type="range"
            min={control.min}
            max={control.max}
            step={control.step}
            value={control.value}
            onInput={(event) => setRow(send, row, Number(event.currentTarget.value))}
          />
          <span class="number small">{control.words}</span>
          <span>{row}</span>
        </label>
      );
    case 'choice':
      return (
        <label class="option-line">
          <select class="field" value={control.index} onChange={(event) => setRow(send, row, Number(event.currentTarget.value))}>
            {control.labels.map((label, at) => (
              <option value={at} key={at}>
                {label}
              </option>
            ))}
          </select>
          <span>{row}</span>
        </label>
      );
    case 'hue':
      return (
        <label class="option-line">
          <HueBox hue={control.hue} changed={(words) => setRow(send, row, words)} swatch={swatch(0)} />
          <span>{row}</span>
        </label>
      );
    case 'text':
      return (
        <label class="option-line">
          <input class="field" value={control.words} onChange={(event) => setRow(send, row, event.currentTarget.value)} />
          <span>{row}</span>
        </label>
      );
    case 'file':
      return (
        <label class="option-line">
          <FileBox row={row} send={send} words={control.words} hint={control.hint} fonts={control.fonts} />
          <span>{row}</span>
        </label>
      );
    case 'lines':
      return (
        <div class="option-list">
          <span>{row}</span>
          <DraftArea words={control.words} hint={control.hint} sent={(words) => setRow(send, row, words)} />
        </div>
      );
    case 'keys':
      return (
        <div class="option-list">
          <span>{row}</span>
          <Keys keys={control} row={row} send={send} remove={remove} />
        </div>
      );
    case 'info_items':
      return (
        <div class="option-list">
          <span>{row}</span>
          {control.items.map((item, at) => (
            <div class="option-line option-entry" key={at}>
              <input class="field" placeholder={control.hint} value={item.label} onChange={(event) => setField(send, row, at, 'label', event.currentTarget.value)} />
              <HueBox hue={item.hue} changed={(words) => setField(send, row, at, 'hue', words)} swatch={swatch(at)} />
              <select class="field" value={item.data} onChange={(event) => setField(send, row, at, 'data', Number(event.currentTarget.value))}>
                {control.data_labels.map((label, index) => (
                  <option value={index} key={index}>
                    {label}
                  </option>
                ))}
              </select>
              <Remove words={remove} row={row} at={at} send={send} />
            </div>
          ))}
          <Add words={control.add} row={row} send={send} />
        </div>
      );
    case 'journal_tabs':
      return (
        <div class="option-list">
          <span>{row}</span>
          {control.tabs.map((tab, at) => (
            <div class="option-entry" key={at}>
              <div class="option-line">
                <input class="field" placeholder={control.hint} value={tab.name} onChange={(event) => setField(send, row, at, 'name', event.currentTarget.value)} />
                <Remove words={remove} row={row} at={at} send={send} />
              </div>
              <div class="option-line wrap">
                {control.kind_labels.map((label, kind) => (
                  <label class="small" key={kind}>
                    <input type="checkbox" checked={tab.kinds[kind]} onChange={(event) => setField(send, row, at, `kind:${kind}`, event.currentTarget.checked)} />
                    {label}
                  </label>
                ))}
              </div>
            </div>
          ))}
          <Add words={control.add} row={row} send={send} />
        </div>
      );
    case 'cooldowns':
      return (
        <div class="option-list">
          <span>{row}</span>
          {control.rules.map((rule, at) => (
            <div class="option-entry" key={at}>
              <div class="option-line">
                <input class="field" placeholder={control.hints[0]} value={rule.label} onChange={(event) => setField(send, row, at, 'label', event.currentTarget.value)} />
                <HueBox hue={rule.hue} changed={(words) => setField(send, row, at, 'hue', words)} swatch={swatch(at)} />
                <input class="field number" type="number" value={rule.seconds} onChange={(event) => setField(send, row, at, 'seconds', Number(event.currentTarget.value))} />
                <Remove words={remove} row={row} at={at} send={send} />
              </div>
              <div class="option-line">
                <input class="field" placeholder={control.hints[1]} value={rule.trigger} onChange={(event) => setField(send, row, at, 'trigger', event.currentTarget.value)} />
                <select class="field" value={rule.source} onChange={(event) => setField(send, row, at, 'source', Number(event.currentTarget.value))}>
                  {control.source_labels.map((label, index) => (
                    <option value={index} key={index}>
                      {label}
                    </option>
                  ))}
                </select>
                <label class="small">
                  <input type="checkbox" checked={rule.restart} onChange={(event) => setField(send, row, at, 'restart', event.currentTarget.checked)} />
                  {control.restart}
                </label>
              </div>
            </div>
          ))}
          <Add words={control.add} row={row} send={send} />
        </div>
      );
    case 'highlights':
      return (
        <div class="option-list">
          <span>{row}</span>
          {control.rules.map((rule, at) => (
            <div class="option-entry" key={at}>
              <div class="option-line">
                <input class="field" placeholder={control.hints[0]} value={rule.name} onChange={(event) => setField(send, row, at, 'name', event.currentTarget.value)} />
                <HueBox hue={rule.hue} changed={(words) => setField(send, row, at, 'hue', words)} swatch={swatch(at)} />
                <label class="small">
                  <input type="checkbox" checked={rule.need_all} onChange={(event) => setField(send, row, at, 'need_all', event.currentTarget.checked)} />
                  {control.need_all}
                </label>
                <label class="small">
                  <input type="checkbox" checked={rule.corpses_only} onChange={(event) => setField(send, row, at, 'corpses_only', event.currentTarget.checked)} />
                  {control.corpses_only}
                </label>
                <Remove words={remove} row={row} at={at} send={send} />
              </div>
              {rule.needs.map((need, place) => (
                <div class="option-line option-need" key={place}>
                  <input
                    class="field"
                    placeholder={control.hints[1]}
                    value={need.words}
                    onChange={(event) => send({ need: { row, at, need: place, name: 'words', value: event.currentTarget.value } })}
                  />
                  <label class="small">
                    <input type="checkbox" checked={need.min !== null} onChange={(event) => send({ need: { row, at, need: place, name: 'at_least', value: event.currentTarget.checked } })} />
                    {control.at_least}
                  </label>
                  {need.min !== null && (
                    <input
                      class="field number"
                      type="number"
                      value={need.min}
                      onChange={(event) => send({ need: { row, at, need: place, name: 'min', value: Number(event.currentTarget.value) } })}
                    />
                  )}
                  <button type="button" class="button small" onClick={() => send({ remove_need: { row, at, need: place } })}>
                    {remove}
                  </button>
                </div>
              ))}
              <button type="button" class="button small" onClick={() => send({ add_need: { row, at } })}>
                {control.add_need}
              </button>
            </div>
          ))}
          <Add words={control.add} row={row} send={send} />
          <div class="button-row">
            {control.presets.map((words, at) => (
              <button type="button" class="button small" key={at} onClick={() => send({ preset: at })}>
                {words}
              </button>
            ))}
          </div>
        </div>
      );
    case 'counter_items':
      return (
        <div class="option-list">
          <span>{row}</span>
          {control.items.map((item, at) => (
            <div class="option-line option-entry" key={at}>
              <input class="field" placeholder={control.hint} value={item.label} onChange={(event) => setField(send, row, at, 'label', event.currentTarget.value)} />
              <input class="field number hue-words" value={item.graphic} onChange={(event) => setField(send, row, at, 'graphic', event.currentTarget.value)} />
              <HueBox hue={item.hue} changed={(words) => setField(send, row, at, 'hue', words)} swatch={swatch(at)} />
              <Remove words={remove} row={row} at={at} send={send} />
            </div>
          ))}
          <Add words={control.add} row={row} send={send} />
        </div>
      );
  }
}

/**
 * The Options: the pages, the rows of the page that shows, and Cancel,
 * Apply, Default, Okay and Save as default. Each change goes to the view,
 * which keeps it on its copy of the profile until Apply or Okay.
 */
export function Options({ data, send }: { data: OptionsData; send: Send }) {
  return (
    <div class="options">
      <div class="options-pages">
        {data.pages.map((page, at) => (
          <button type="button" class={`button row${page.chosen ? ' chosen' : ''}`} key={at} onClick={() => send({ page: at })}>
            {page.words}
          </button>
        ))}
      </div>
      <div class="options-main">
        <div class="options-rows">
          {data.rows.map((row) => (
            <div class="option-row" key={row.label}>
              {row.section && <h2 class="heading small goal-words">{row.section}</h2>}
              <RowControl control={row.control} row={row.label} send={send} remove={data.remove} />
            </div>
          ))}
        </div>
        <div class="button-row options-foot">
          {data.foot.map((foot, at) => (
            <button type="button" class="button" key={at} title={at === data.default_at ? data.default_hint : undefined} style={{ color: foot.color }} onClick={() => send({ foot: at })}>
              {foot.words}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
