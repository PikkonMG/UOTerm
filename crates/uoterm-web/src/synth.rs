//! MIDI music for the browser, for the clients that have no MP3 music. A
//! software synth plays the whole MIDI file with the SoundFont the server
//! sends, into samples the page plays as one sound buffer. Which file of a
//! track plays is `uoterm_view::audio::music_file`.

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

const STEREO: usize = 2;
/// The synth makes this many frames at a time.
const RENDER_FRAMES: usize = 2048;
/// A track is cut after this long, so a broken file cannot fill the memory
/// of the page.
const MOST_SECONDS: usize = 600;

/// The samples of a MIDI file played once with a SoundFont at
/// `sample_rate`: left and right in turn. Empty when the file, the font or
/// the rate cannot be read.
pub fn render(midi: &[u8], sound_font: &[u8], sample_rate: u32) -> Vec<f32> {
    let (Ok(font), Ok(midi), Ok(rate)) = (
        SoundFont::new(&mut &sound_font[..]),
        MidiFile::new(&mut &midi[..]),
        i32::try_from(sample_rate),
    ) else {
        return Vec::new();
    };
    let settings = SynthesizerSettings::new(rate);
    let Ok(synthesizer) = Synthesizer::new(&Arc::new(font), &settings) else {
        return Vec::new();
    };
    let mut sequencer = MidiFileSequencer::new(synthesizer);
    sequencer.play(&Arc::new(midi), false);
    let most = sample_rate as usize * MOST_SECONDS * STEREO;
    let (mut left, mut right) = (vec![0.0; RENDER_FRAMES], vec![0.0; RENDER_FRAMES]);
    let mut samples = Vec::new();
    while !sequencer.end_of_sequence() && samples.len() < most {
        sequencer.render(&mut left, &mut right);
        samples.extend(left.iter().zip(&right).flat_map(|(l, r)| [*l, *r]));
    }
    samples
}

/// The samples of a MIDI file, for the page: left and right in turn.
#[wasm_bindgen(js_name = renderMidi)]
pub fn render_midi(midi: &[u8], sound_font: &[u8], sample_rate: u32) -> js_sys::Float32Array {
    js_sys::Float32Array::from(render(midi, sound_font, sample_rate).as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A SoundFont for the tests that need one. They skip when it is unset.
    const ENV_TEST_SOUNDFONT: &str = "UOTERM_TEST_SOUNDFONT";
    const RATE: u32 = 22_050;

    /// One note of one beat, in a MIDI file of one track.
    const ONE_NOTE: [u8; 34] = [
        b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, // header
        b'M', b'T', b'r', b'k', 0, 0, 0, 12, // track
        0, 0x90, 60, 100, // note on
        96, 0x80, 60, 0, // note off after one beat
        0, 0xFF, 0x2F, 0, // end
    ];

    #[test]
    fn a_font_that_cannot_be_read_gives_no_sound() {
        assert!(render(&ONE_NOTE, b"not a sound font", RATE).is_empty());
    }

    #[test]
    fn a_note_sounds_in_both_ears_and_ends() {
        let Ok(path) = std::env::var(ENV_TEST_SOUNDFONT) else {
            return;
        };
        let font = std::fs::read(path).unwrap();
        let samples = render(&ONE_NOTE, &font, RATE);
        assert_eq!(samples.len() % (RENDER_FRAMES * STEREO), 0);
        assert!(samples.iter().any(|sample| sample.abs() > 0.0));
        assert!(samples.len() < RATE as usize * STEREO * 2, "about one beat");
    }
}
