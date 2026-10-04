import type { LauncherData, Send } from './types';

/** The panel launcher: a button for each panel it opens and closes, lit while the panel shows. */
export function Launcher({ data, send }: { data: LauncherData; send: Send }) {
  return (
    <div class="launcher">
      {data.buttons.map((button, at) => (
        <button type="button" key={at} class={`button segment${button.shows ? ' chosen' : ''}`} onClick={() => send({ toggle: at })}>
          {button.words}
        </button>
      ))}
    </div>
  );
}
