/**
 * The sound device of the page, with Web Audio. The view decides every
 * sound (`audioOut()`, the rules of `uoterm_view::audio`): which effect
 * starts as which voice and which voice it replaces, the music, the rain,
 * and how loud each plays by the Sound page and the focus. The player only
 * plays: it fetches each sound once, and tells the view when a voice ended
 * so its room goes to the next sound.
 */

import { answerOf, Music, type RenderMidi } from './music';

/** What the view asks of the sound device (`AudioOut` of uoterm-view). */
export type AudioOut =
  | { kind: 'Effect'; voice: number; sound: number; volume: number; replace: number | null }
  | { kind: 'Music'; track: number; volume: number; sound_font: string | null }
  | { kind: 'MusicStop' }
  | { kind: 'Rain'; sound: number; volume: number }
  | { kind: 'RainStop' }
  | { kind: 'Volumes'; voices: { voice: number; volume: number }[]; music: number; rain: number };

export interface PlayerPlaces {
  /** Tells the view that a voice came to its end, or could not play. */
  ended(voice: number): void;
  /** Where the sound font the profile names comes from. */
  soundFontPath: string;
  renderMidi: RenderMidi;
}

const SOUND_PATH = '/v1/sound/';
/** The context plays only while running; it starts suspended until the player presses something. */
const RUNNING = 'running';
/** The presses that let a page make sound, as browsers ask. */
const GESTURES = ['pointerdown', 'keydown'];

/** One sound that plays (a voice, or the loop of the rain), with the gain it goes through; no source while it loads. */
interface Playing {
  gain: GainNode;
  source: AudioBufferSourceNode | null;
}

/**
 * Resumes `context` on the first press of the player on `target`: the
 * browser lets a page make sound only after one. Gives the function that
 * stops listening.
 */
export function resumeOnGesture(target: EventTarget, context: AudioContext): () => void {
  const stop = () => {
    for (const gesture of GESTURES) target.removeEventListener(gesture, resume);
  };
  const resume = () => {
    stop();
    void context.resume().catch(() => {});
  };
  for (const gesture of GESTURES) target.addEventListener(gesture, resume);
  return stop;
}

export class Player {
  /** The decoded sounds by number; null for a sound the server lacks. */
  private readonly sounds = new Map<number, Promise<AudioBuffer | null>>();
  private readonly voices = new Map<number, Playing>();
  private rain: Playing | null = null;
  private readonly music: Music;

  constructor(
    private readonly context: AudioContext,
    private readonly places: PlayerPlaces,
  ) {
    this.music = new Music(context, places);
  }

  /** Does what the view asked, in order. */
  play(outs: AudioOut[]): void {
    for (const out of outs) {
      switch (out.kind) {
        case 'Effect':
          if (out.replace !== null) this.stopVoice(out.replace);
          this.startVoice(out.voice, out.sound, out.volume);
          break;
        case 'Music':
          this.music.play(out.track, out.volume, out.sound_font);
          break;
        case 'MusicStop':
          this.music.stop();
          break;
        case 'Rain':
          this.startRain(out.sound, out.volume);
          break;
        case 'RainStop':
          this.stopRain();
          break;
        case 'Volumes':
          for (const { voice, volume } of out.voices) {
            const playing = this.voices.get(voice);
            if (playing) playing.gain.gain.value = volume;
          }
          this.music.setVolume(out.music);
          if (this.rain) this.rain.gain.gain.value = out.rain;
          break;
      }
    }
  }

  /** Stops every sound and lets the context go. */
  close(): void {
    for (const voice of [...this.voices.keys()]) this.stopVoice(voice);
    this.stopRain();
    this.music.stop();
    void this.context.close().catch(() => {});
  }

  /** The decoded sound `sound`, fetched once. */
  private sound(sound: number): Promise<AudioBuffer | null> {
    let decoded = this.sounds.get(sound);
    if (!decoded) {
      decoded = answerOf(`${SOUND_PATH}${sound}`).then(async (answer) => {
        if (!answer) return null;
        try {
          return await this.context.decodeAudioData(await answer.arrayBuffer());
        } catch {
          return null;
        }
      });
      this.sounds.set(sound, decoded);
    }
    return decoded;
  }

  /** A gain of `volume` into the speakers. */
  private gainOf(volume: number): GainNode {
    const gain = this.context.createGain();
    gain.gain.value = volume;
    gain.connect(this.context.destination);
    return gain;
  }

  /**
   * Starts `sound` as `voice`. A context that does not run yet would keep
   * the sound and play it late, all at once with the others: the sound is
   * left out, and its voice ends at once.
   */
  private startVoice(voice: number, sound: number, volume: number): void {
    if (this.context.state !== RUNNING) {
      this.places.ended(voice);
      return;
    }
    const playing: Playing = { gain: this.gainOf(volume), source: null };
    this.voices.set(voice, playing);
    void this.sound(sound).then((buffer) => {
      // The voice was replaced while its sound loaded.
      if (this.voices.get(voice) !== playing) return;
      if (!buffer) {
        this.endVoice(voice);
        return;
      }
      const source = this.context.createBufferSource();
      source.buffer = buffer;
      source.connect(playing.gain);
      source.onended = () => this.endVoice(voice);
      source.start();
      playing.source = source;
    });
  }

  /** The voice came to its end: the view hears of it. */
  private endVoice(voice: number): void {
    const playing = this.voices.get(voice);
    if (!playing) return;
    this.voices.delete(voice);
    playing.gain.disconnect();
    this.places.ended(voice);
  }

  /** Stops a voice the view replaced; the view knows it is gone. */
  private stopVoice(voice: number): void {
    const playing = this.voices.get(voice);
    if (!playing) return;
    this.voices.delete(voice);
    if (playing.source) {
      playing.source.onended = null;
      playing.source.stop();
    }
    playing.gain.disconnect();
  }

  private startRain(sound: number, volume: number): void {
    this.stopRain();
    const rain: Playing = { gain: this.gainOf(volume), source: null };
    this.rain = rain;
    void this.sound(sound).then((buffer) => {
      if (!buffer || this.rain !== rain) return;
      const source = this.context.createBufferSource();
      source.buffer = buffer;
      source.loop = true;
      source.connect(rain.gain);
      source.start();
      rain.source = source;
    });
  }

  private stopRain(): void {
    this.rain?.source?.stop();
    this.rain?.gain.disconnect();
    this.rain = null;
  }
}
