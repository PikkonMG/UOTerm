/**
 * The music of the page: one track at a time. The server picks the file of
 * a track by the rule of the Rust window (`music_file` of the view): a MIDI
 * file only when the page has a sound font, else the MP3 file. An MP3 file
 * plays as the browser decodes it; a MIDI file is played by the synth of
 * the view (`renderMidi`) with the sound font of the profile, into one
 * buffer. A header says whether the track starts again at its end.
 */

/** Plays a MIDI file with a sound font: left and right samples in turn. */
export type RenderMidi = (midi: Uint8Array, soundFont: Uint8Array, sampleRate: number) => Float32Array;

const MUSIC_PATH = '/v1/music/';
const MIDI_QUERY = '?midi=true';
const CONTENT_TYPE = 'content-type';
const MIDI_TYPE = 'audio/midi';
const REPEATS_HEADER = 'x-uoterm-repeats';
const REPEATS = 'true';
const STEREO = 2;

/** The answer of `path`; null when the server has none, or no answer came. */
export async function answerOf(path: string): Promise<Response | null> {
  try {
    const answer = await fetch(path);
    return answer.ok ? answer : null;
  } catch {
    return null;
  }
}

/** The samples of left and right in turn as a buffer of two channels. */
function stereoBuffer(context: AudioContext, samples: Float32Array): AudioBuffer | null {
  const frames = samples.length / STEREO;
  if (frames < 1) return null;
  const buffer = context.createBuffer(STEREO, frames, context.sampleRate);
  for (let channel = 0; channel < STEREO; channel += 1) {
    buffer.copyToChannel(
      samples.filter((_, at) => at % STEREO === channel),
      channel,
    );
  }
  return buffer;
}

export interface MusicPlaces {
  /** Where the sound font the profile names comes from. */
  soundFontPath: string;
  renderMidi: RenderMidi;
}

export class Music {
  private readonly gain: GainNode;
  private source: AudioBufferSourceNode | null = null;
  /** Counts the tracks asked for: a track that loads after a newer ask does not play. */
  private asked = 0;
  /** The sound font of the profile, by the file it names; null when it does not read. */
  private font: { name: string; bytes: Promise<Uint8Array | null> } | null = null;

  constructor(
    private readonly context: AudioContext,
    private readonly places: MusicPlaces,
  ) {
    this.gain = context.createGain();
    this.gain.connect(context.destination);
  }

  /** Plays `track` in place of the music before, `volume` loud. `soundFont` names the sound font of the profile, if any. */
  play(track: number, volume: number, soundFont: string | null): void {
    this.stop();
    this.setVolume(volume);
    const ask = this.asked;
    void this.load(track, soundFont).then((loaded) => {
      if (!loaded || ask !== this.asked) return;
      const source = this.context.createBufferSource();
      source.buffer = loaded.buffer;
      source.loop = loaded.repeats;
      source.connect(this.gain);
      source.start();
      this.source = source;
    });
  }

  stop(): void {
    this.asked += 1;
    this.source?.stop();
    this.source = null;
  }

  setVolume(volume: number): void {
    this.gain.gain.value = volume;
  }

  /** The buffer of `track` and whether it repeats; null when it does not play. */
  private async load(track: number, soundFont: string | null): Promise<{ buffer: AudioBuffer; repeats: boolean } | null> {
    const font = soundFont === null ? null : await this.soundFont(soundFont);
    const answer = await answerOf(`${MUSIC_PATH}${track}${font ? MIDI_QUERY : ''}`);
    if (!answer) return null;
    const repeats = answer.headers.get(REPEATS_HEADER) === REPEATS;
    const bytes = await answer.arrayBuffer();
    try {
      const buffer =
        answer.headers.get(CONTENT_TYPE) === MIDI_TYPE && font
          ? stereoBuffer(this.context, this.places.renderMidi(new Uint8Array(bytes), font, this.context.sampleRate))
          : await this.context.decodeAudioData(bytes);
      return buffer ? { buffer, repeats } : null;
    } catch {
      return null;
    }
  }

  /** The bytes of the sound font `name`, fetched again only when the profile names another. */
  private soundFont(name: string): Promise<Uint8Array | null> {
    if (this.font?.name !== name) {
      const bytes = answerOf(this.places.soundFontPath).then(async (answer) => (answer ? new Uint8Array(await answer.arrayBuffer()) : null));
      this.font = { name, bytes };
    }
    return this.font.bytes;
  }
}
