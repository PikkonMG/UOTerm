import type { InviteData, Send } from './types';

/** The box of a party invite, while the party tab is closed: the words of the shard, and Accept and Decline while the human has control. */
export function PartyInvite({ data, send }: { data: InviteData; send: Send }) {
  return (
    <div class="invite">
      <p class="small">{data.words}</p>
      {data.accept && data.decline && (
        <div class="button-row">
          <button type="button" class="button" style={{ color: data.accept.color }} onClick={() => send({ accept: true })}>
            {data.accept.words}
          </button>
          <button type="button" class="button" style={{ color: data.decline.color }} onClick={() => send({ decline: true })}>
            {data.decline.words}
          </button>
        </div>
      )}
    </div>
  );
}
