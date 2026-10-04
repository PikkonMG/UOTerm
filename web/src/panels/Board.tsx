import type { BoardData, Send } from './types';

/**
 * The bulletin board: the messages with each answer under the one it
 * answers, the message that is read on paper, and the fields and buttons
 * that post, reply, remove and close.
 */
export function Board({ data, send }: { data: BoardData; send: Send }) {
  return (
    <div class="board">
      <div class="board-posts">
        {data.posts.map((post) => (
          <button
            type="button"
            class={`button row board-post${post.reading ? ' chosen' : ''}`}
            key={post.serial}
            style={{ marginLeft: `${post.indent}px` }}
            disabled={!data.live}
            onClick={() => send({ read: post.serial })}
          >
            <span>{post.subject}</span>
            <span class="small faint">{post.poster}</span>
          </button>
        ))}
      </div>
      <div class="board-read">
        <div class="paper board-text" style={{ background: data.paper, color: data.ink }}>
          {data.text}
        </div>
        {data.live && (
          <>
            <input class="field" placeholder={data.subject_hint} value={data.subject} onInput={(event) => send({ subject: event.currentTarget.value })} />
            <textarea class="field board-write" placeholder={data.text_hint} value={data.body} onInput={(event) => send({ text: event.currentTarget.value })} />
            <div class="button-row">
              <button type="button" class="button goal" onClick={() => send({ post: true })}>
                {data.post}
              </button>
              {data.reply && (
                <button type="button" class="button" onClick={() => send({ reply: true })}>
                  {data.reply}
                </button>
              )}
              {data.remove && (
                <button type="button" class="button alarm" onClick={() => send({ remove: true })}>
                  {data.remove}
                </button>
              )}
              <button type="button" class="button dim" onClick={() => send({ close: true })}>
                {data.close}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
