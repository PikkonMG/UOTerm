import { useLayoutEffect, useRef } from 'preact/hooks';
import type { BookData, Send } from './types';

/**
 * One page of a book the player writes: what he types goes to the view,
 * which breaks a line too wide for the page and keeps a full page from
 * taking more; the page then shows the words the view kept, with the
 * caret where the view put it.
 */
function WrittenPage({ page, side, edits, lines, send }: { page: BookData['pages'][number]; side: number; edits: number; lines: number; send: Send }) {
  const field = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const target = field.current;
    if (!target) return;
    if (target.value !== page.words) target.value = page.words;
    if (page.caret !== null && document.activeElement === target) target.setSelectionRange(page.caret, page.caret);
  }, [edits, page.words, page.caret]);
  return (
    <textarea
      ref={field}
      class="field book-page"
      rows={lines}
      value={page.words}
      onInput={(event) => send({ page: { side, words: event.currentTarget.value, caret: event.currentTarget.selectionStart } })}
    />
  );
}

/**
 * The open book, two pages at a time: in a book the player writes, the
 * cover and the pages take words; a sealed one shows its words on paper.
 * The buttons turn the pages, save and close.
 */
export function Book({ data, send }: { data: BookData; send: Send }) {
  const paper = { background: data.paper, color: data.ink };
  return (
    <div class="book">
      {data.writing ? (
        <div class="book-cover">
          <input class="field" placeholder={data.title_hint} value={data.title} onInput={(event) => send({ cover: { title: event.currentTarget.value, author: data.author } })} />
          <input class="field" placeholder={data.author_hint} value={data.author} onInput={(event) => send({ cover: { title: data.title, author: event.currentTarget.value } })} />
        </div>
      ) : (
        <p class="dim">{data.by}</p>
      )}
      <div class="book-pages">
        {data.pages.map((page, side) => (
          <div class="book-side" key={page.number}>
            {data.writing ? (
              <WrittenPage page={page} side={side} edits={data.edits} lines={data.lines} send={send} />
            ) : (
              <div class="book-page paper" style={paper}>
                {page.words}
              </div>
            )}
            <span class="number small faint">{page.number}</span>
          </div>
        ))}
      </div>
      <div class="button-row">
        {data.turns.map((words, at) => (
          <button type="button" class="button" key={at} onClick={() => send({ turn: at })}>
            {words}
          </button>
        ))}
        {data.save && (
          <button type="button" class="button goal" onClick={() => send({ save: true })}>
            {data.save}
          </button>
        )}
        {data.close && (
          <button type="button" class="button dim" onClick={() => send({ close: true })}>
            {data.close}
          </button>
        )}
      </div>
    </div>
  );
}
