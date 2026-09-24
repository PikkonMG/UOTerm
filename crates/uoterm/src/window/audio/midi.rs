//! MIDI music, for the clients that have no MP3 music. A software synth
//! plays the MIDI file with the SoundFont the player set on the Sound page.

use rodio::Source;
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::io::BufReader;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uoterm_nav::MusicTrack;

const MIDI_SAMPLE_RATE: u32 = 44_100;
const STEREO: usize = 2;
const MIDI_CHANNELS: NonZero<u16> = NonZero::new(STEREO as u16).unwrap();
const MIDI_RATE: NonZero<u32> = NonZero::new(MIDI_SAMPLE_RATE).unwrap();
/// The synth makes this many frames at a time.
const RENDER_FRAMES: usize = 2048;

/// The file of a track that plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicFile<'a> {
    Mp3(&'a Path),
    Midi(&'a Path),
}

/// The file of `track` to play. A MIDI file plays only with a SoundFont.
/// When `Config.txt` names the MIDI file, it plays before the MP3 file;
/// else the MP3 file plays first.
pub fn music_file(track: &MusicTrack, has_sound_font: bool) -> Option<MusicFile<'_>> {
    let midi = track
        .midi
        .as_deref()
        .filter(|_| has_sound_font)
        .map(MusicFile::Midi);
    let mp3 = track.mp3.as_deref().map(MusicFile::Mp3);
    if track.midi_first {
        midi.or(mp3)
    } else {
        mp3.or(midi)
    }
}

/// The last SoundFont read, with the file it came from. A file that could
/// not be read is kept as None, so it is not read again each frame.
#[derive(Default)]
pub struct SoundFonts {
    kept: Option<(PathBuf, Option<Arc<SoundFont>>)>,
}

impl SoundFonts {
    pub fn get(&mut self, path: &Path) -> Option<Arc<SoundFont>> {
        if self.kept.as_ref().is_none_or(|(kept, _)| kept != path) {
            let font = File::open(path)
                .ok()
                .and_then(|file| SoundFont::new(&mut BufReader::new(file)).ok())
                .map(Arc::new);
            self.kept = Some((path.to_path_buf(), font));
        }
        self.kept.as_ref().and_then(|(_, font)| font.clone())
    }
}

/// A MIDI file as sound: left and right samples in turn.
pub struct MidiSource {
    sequencer: MidiFileSequencer,
    left: Vec<f32>,
    right: Vec<f32>,
    /// The next sample of the frames made, left and right in turn.
    next: usize,
}

impl MidiSource {
    /// Starts the MIDI file at `path`. None when the file or the synth fails.
    pub fn open(path: &Path, font: &Arc<SoundFont>, repeats: bool) -> Option<Self> {
        let mut reader = BufReader::new(File::open(path).ok()?);
        Self::start(Arc::new(MidiFile::new(&mut reader).ok()?), font, repeats)
    }

    fn start(midi: Arc<MidiFile>, font: &Arc<SoundFont>, repeats: bool) -> Option<Self> {
        let settings = SynthesizerSettings::new(MIDI_SAMPLE_RATE as i32);
        let mut sequencer = MidiFileSequencer::new(Synthesizer::new(font, &settings).ok()?);
        sequencer.play(&midi, repeats);
        Some(Self {
            sequencer,
            left: vec![0.0; RENDER_FRAMES],
            right: vec![0.0; RENDER_FRAMES],
            next: RENDER_FRAMES * STEREO,
        })
    }
}

impl Iterator for MidiSource {
    type Item = f32;

    /// The next sample. A file that does not repeat ends after its last
    /// note; a file that repeats never ends.
    fn next(&mut self) -> Option<f32> {
        if self.next == self.left.len() * STEREO {
            if self.sequencer.end_of_sequence() {
                return None;
            }
            self.sequencer.render(&mut self.left, &mut self.right);
            self.next = 0;
        }
        let frame = self.next / STEREO;
        let sample = if self.next.is_multiple_of(STEREO) {
            self.left[frame]
        } else {
            self.right[frame]
        };
        self.next += 1;
        Some(sample)
    }
}

impl Source for MidiSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        MIDI_CHANNELS
    }

    fn sample_rate(&self) -> NonZero<u32> {
        MIDI_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A SoundFont for the tests that need one. They skip when it is unset.
    const ENV_TEST_SOUNDFONT: &str = "UOTERM_TEST_SOUNDFONT";

    fn track(mp3: bool, midi: bool, midi_first: bool) -> MusicTrack {
        MusicTrack {
            mp3: mp3.then(|| PathBuf::from("a.mp3")),
            midi: midi.then(|| PathBuf::from("a.mid")),
            midi_first,
            repeats: false,
        }
    }

    #[test]
    fn midi_plays_only_with_a_sound_font_and_mp3_is_the_fallback() {
        let mp3 = Some(MusicFile::Mp3(Path::new("a.mp3")));
        let midi = Some(MusicFile::Midi(Path::new("a.mid")));
        assert_eq!(music_file(&track(true, true, true), true), midi);
        assert_eq!(music_file(&track(true, true, true), false), mp3);
        assert_eq!(music_file(&track(true, true, false), true), mp3);
        assert_eq!(music_file(&track(false, true, false), true), midi);
        assert_eq!(music_file(&track(false, true, false), false), None);
        assert_eq!(music_file(&track(true, false, true), true), mp3);
    }

    #[test]
    fn a_sound_font_that_cannot_be_read_gives_none() {
        let mut fonts = SoundFonts::default();
        assert!(fonts.get(Path::new("/no/such/font.sf2")).is_none());
        assert!(fonts.get(Path::new("/no/such/font.sf2")).is_none());
    }

    /// One note of half a second, in a MIDI file of one track.
    fn one_note() -> Arc<MidiFile> {
        const MIDI: [u8; 34] = [
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, // header
            b'M', b'T', b'r', b'k', 0, 0, 0, 12, // track
            0, 0x90, 60, 100, // note on
            96, 0x80, 60, 0, // note off after one beat
            0, 0xFF, 0x2F, 0, // end
        ];
        Arc::new(MidiFile::new(&mut &MIDI[..]).unwrap())
    }

    #[test]
    fn midi_sounds_and_ends_unless_it_repeats() {
        let Ok(path) = std::env::var(ENV_TEST_SOUNDFONT) else {
            return;
        };
        let font = SoundFonts::default().get(Path::new(&path)).unwrap();
        let once = MidiSource::start(one_note(), &font, false).unwrap();
        let samples: Vec<f32> = once.collect();
        assert!(samples.iter().any(|s| s.abs() > 0.0));
        let second = (MIDI_SAMPLE_RATE as usize) * STEREO;
        assert!(samples.len() < second * 2);
        let repeating = MidiSource::start(one_note(), &font, true).unwrap();
        assert_eq!(repeating.take(second * 3).count(), second * 3);
    }
}
