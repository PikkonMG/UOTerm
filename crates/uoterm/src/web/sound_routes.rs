//! The sounds of the client files as WAV files, the music as the MP3 or
//! MIDI file of a track, and the MIDI sound font the profile names.

use super::profile_routes::profile_of;
use super::{kept, on_blocking, send_file, WebState};
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use std::io::Read;
use uoterm_nav::SOUND_SAMPLE_RATE;
use uoterm_view::audio::{music_file, MusicFile};

const CONTENT_WAV: &str = "audio/wav";
const CONTENT_MP3: &str = "audio/mpeg";
const CONTENT_MIDI: &str = "audio/midi";
const CONTENT_SOUND_FONT: &str = "application/octet-stream";
/// Whether a track starts again at its end.
const REPEATS_HEADER: &str = "x-uoterm-repeats";
/// A WAV file: a RIFF file of the WAVE kind, a format part and a data part.
const RIFF_MARK: &[u8; 4] = b"RIFF";
const WAVE_MARK: &[u8; 4] = b"WAVE";
const FORMAT_MARK: &[u8; 4] = b"fmt ";
const DATA_MARK: &[u8; 4] = b"data";
/// The bytes of the RIFF file after its length field, before the data.
const WAVE_HEAD_AFTER_LENGTH: u32 = 36;
const FORMAT_BYTES: u32 = 16;
const FORMAT_PCM: u16 = 1;
const ONE_CHANNEL: u16 = 1;
const SAMPLE_BYTES: u16 = 2;
const SAMPLE_BITS: u16 = 16;
/// A sound font is a RIFF file of the `sfbk` kind.
const SOUND_FONT_MARK: &[u8; 4] = b"sfbk";
const RIFF_KIND_AT: usize = 8;
const RIFF_HEAD_BYTES: usize = 12;

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route("/v1/sound/{id}", get(sound))
        .route("/v1/music/{id}", get(music))
        .route("/v1/soundfont", get(sound_font))
}

/// Samples of one channel of 16 bits as a WAV file.
fn wave_file(samples: &[i16]) -> Vec<u8> {
    let data_bytes = (samples.len() * usize::from(SAMPLE_BYTES)) as u32;
    let mut wave =
        Vec::with_capacity(RIFF_HEAD_BYTES + WAVE_HEAD_AFTER_LENGTH as usize + data_bytes as usize);
    wave.extend(RIFF_MARK);
    wave.extend((WAVE_HEAD_AFTER_LENGTH + data_bytes).to_le_bytes());
    wave.extend(WAVE_MARK);
    wave.extend(FORMAT_MARK);
    wave.extend(FORMAT_BYTES.to_le_bytes());
    wave.extend(FORMAT_PCM.to_le_bytes());
    wave.extend(ONE_CHANNEL.to_le_bytes());
    wave.extend(SOUND_SAMPLE_RATE.to_le_bytes());
    wave.extend((SOUND_SAMPLE_RATE * u32::from(SAMPLE_BYTES)).to_le_bytes());
    wave.extend(SAMPLE_BYTES.to_le_bytes());
    wave.extend(SAMPLE_BITS.to_le_bytes());
    wave.extend(DATA_MARK);
    wave.extend(data_bytes.to_le_bytes());
    wave.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
    wave
}

/// One sound as a WAV file the browser keeps for this version of the
/// client files. Unavailable when the client files hold no sounds.
async fn sound(State(state): State<WebState>, Path(id): Path<u16>) -> Response {
    let Some(sounds) = state.sounds.clone() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    on_blocking(move || match sounds.samples(id) {
        Some(samples) => kept(&state.files_tag, CONTENT_WAV, wave_file(&samples)),
        None => StatusCode::NOT_FOUND.into_response(),
    })
    .await
}

#[derive(Deserialize)]
struct MusicQuery {
    /// The page has a sound font, so it can play a MIDI file.
    #[serde(default)]
    midi: bool,
}

/// The file of one track, by the rule of the window: a MIDI file only for
/// a page with a sound font. A header says if the track starts again at
/// its end. Unavailable when the client files hold no music.
async fn music(
    State(state): State<WebState>,
    Path(id): Path<u16>,
    Query(query): Query<MusicQuery>,
    request: Request,
) -> Response {
    let Some(list) = state.music.as_ref() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(track) = list.track(id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (path, content_type) = match music_file(track, query.midi) {
        Some(MusicFile::Mp3(path)) => (path, CONTENT_MP3),
        Some(MusicFile::Midi(path)) => (path, CONTENT_MIDI),
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut answer = send_file(path, request, Some(content_type)).await;
    answer.headers_mut().insert(
        REPEATS_HEADER,
        HeaderValue::from_static(if track.repeats { "true" } else { "false" }),
    );
    answer
}

#[derive(Deserialize)]
struct SoundFontQuery {
    /// The shard and the character whose profile names the sound font.
    /// With neither, the default profile names it.
    shard: Option<String>,
    character: Option<String>,
}

/// True when the file at `path` starts as a sound font does.
fn is_sound_font(path: &std::path::Path) -> bool {
    let mut head = [0u8; RIFF_HEAD_BYTES];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .is_ok_and(|()| {
            head.starts_with(RIFF_MARK) && head[RIFF_KIND_AT..].starts_with(SOUND_FONT_MARK)
        })
}

/// The file at `path` when it is inside one of `folders` once every link
/// and `..` in the names is followed, as that real path. None for a file
/// elsewhere, or one that is not there.
fn inside(path: &std::path::Path, folders: &[&std::path::Path]) -> Option<std::path::PathBuf> {
    let path = path.canonicalize().ok()?;
    folders
        .iter()
        .filter_map(|folder| folder.canonicalize().ok())
        .any(|folder| path.starts_with(folder))
        .then_some(path)
}

/// The MIDI sound font the profile names. Not found when it names none,
/// names a file that is no sound font, or one outside the config folder
/// and the folder of the client files: a profile file must not open any
/// file of this machine to the page.
async fn sound_font(
    State(state): State<WebState>,
    Query(query): Query<SoundFontQuery>,
    request: Request,
) -> Response {
    let config_dir = state.config_dir.clone();
    let client_files = state.client_files.clone();
    let chosen = tokio::task::spawn_blocking(move || {
        let profile = profile_of(
            &config_dir,
            query.shard.as_deref(),
            query.character.as_deref(),
        )?;
        let mut folders = vec![config_dir.as_path()];
        folders.extend(client_files.as_deref());
        Ok::<_, StatusCode>(
            profile
                .sound
                .midi_sound_font
                .and_then(|path| inside(&path, &folders))
                .filter(|path| is_sound_font(path)),
        )
    })
    .await;
    match chosen {
        Ok(Ok(Some(path))) => send_file(&path, request, Some(CONTENT_SOUND_FONT)).await,
        Ok(Ok(None)) => StatusCode::NOT_FOUND.into_response(),
        Ok(Err(status)) => status.into_response(),
        Err(error) => {
            tracing::warn!(%error, "a web route failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fixture_uopath, send, state_in, test_state};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use uoterm_nav::fixtures::{FIXTURE_SOUND_ID, FIXTURE_SOUND_SAMPLES};
    use uoterm_nav::SOUND_SAMPLE_RATE;
    use uoterm_view::settings::Profile;

    const SAMPLE_RATE_AT: usize = 24;
    const DATA_AT: usize = 44;
    const MUSIC_CONFIG: &str = "Music/Digital/Config.txt";
    const TOWN_MP3: &str = "Music/Digital/Town.mp3";
    const OLD_MIDI: &str = "Music/old.mid";
    const SOUND_FONT_BYTES: &[u8] = b"RIFF\x04\x00\x00\x00sfbk";

    fn get(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    async fn body(answer: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn a_sound_comes_as_a_wave_file() {
        let (state, _files) = test_state();
        let path = format!("/v1/sound/{FIXTURE_SOUND_ID}");
        let answer = send(state, get(&path)).await;
        assert_eq!(answer.status(), StatusCode::OK);
        assert_eq!(answer.headers()["content-type"], "audio/wav");
        let bytes = body(answer).await;
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let rate = u32::from_le_bytes(
            bytes[SAMPLE_RATE_AT..SAMPLE_RATE_AT + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(rate, SOUND_SAMPLE_RATE);
        let samples: Vec<i16> = bytes[DATA_AT..]
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(samples, FIXTURE_SOUND_SAMPLES);
    }

    #[tokio::test]
    async fn a_sound_the_files_do_not_hold_is_not_found() {
        let (state, _files) = test_state();
        let path = format!("/v1/sound/{}", FIXTURE_SOUND_ID + 1);
        let answer = send(state, get(&path)).await;
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn music_comes_as_mp3_or_as_midi_for_a_page_with_a_sound_font() {
        let files = fixture_uopath();
        std::fs::create_dir_all(files.0.join("Music/Digital")).unwrap();
        std::fs::write(files.0.join(MUSIC_CONFIG), "1 Town.mp3,loop\n2 old.mid\n").unwrap();
        std::fs::write(files.0.join(TOWN_MP3), b"mp3").unwrap();
        std::fs::write(files.0.join(OLD_MIDI), b"midi").unwrap();
        let state = state_in(Some(&files.0), files.0.join("config"));
        let town = send(state.clone(), get("/v1/music/1")).await;
        assert_eq!(town.status(), StatusCode::OK);
        assert_eq!(town.headers()["content-type"], "audio/mpeg");
        assert_eq!(town.headers()["x-uoterm-repeats"], "true");
        assert_eq!(body(town).await, b"mp3");
        let no_font = send(state.clone(), get("/v1/music/2")).await;
        assert_eq!(no_font.status(), StatusCode::NOT_FOUND);
        let old = send(state.clone(), get("/v1/music/2?midi=true")).await;
        assert_eq!(old.status(), StatusCode::OK);
        assert_eq!(old.headers()["content-type"], "audio/midi");
        assert_eq!(old.headers()["x-uoterm-repeats"], "false");
        let none = send(state, get("/v1/music/3")).await;
        assert_eq!(none.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_sound_font_is_the_one_the_profile_names() {
        let (state, files) = test_state();
        let font = files.0.join("music.sf2");
        let missing = send(state.clone(), get("/v1/soundfont")).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND, "no font is set");
        std::fs::write(&font, SOUND_FONT_BYTES).unwrap();
        let mut profile = Profile::default();
        profile.sound.midi_sound_font = Some(font.clone());
        crate::window::ProfileStore::in_folder(&state.config_dir).save_default(&profile);
        let answer = send(state.clone(), get("/v1/soundfont")).await;
        assert_eq!(answer.status(), StatusCode::OK);
        assert_eq!(body(answer).await, SOUND_FONT_BYTES);
        let character = "/v1/soundfont?shard=127.0.0.1:2593&character=Mara";
        let answer = send(state.clone(), get(character)).await;
        assert_eq!(
            answer.status(),
            StatusCode::OK,
            "the default of a new character"
        );
        let half = send(state.clone(), get("/v1/soundfont?character=Mara")).await;
        assert_eq!(half.status(), StatusCode::BAD_REQUEST);
        std::fs::write(&font, b"not a sound font").unwrap();
        let other = send(state, get("/v1/soundfont")).await;
        assert_eq!(other.status(), StatusCode::NOT_FOUND, "only a sound font");
    }

    /// A profile file can name any path; only a font in the config folder
    /// or in the folder of the client files is sent.
    #[tokio::test]
    async fn a_sound_font_outside_the_known_folders_is_not_found() {
        let (state, _files) = test_state();
        let elsewhere = super::super::tests::temp_folder();
        let outside = elsewhere.0.join("music.sf2");
        std::fs::write(&outside, SOUND_FONT_BYTES).unwrap();
        std::fs::create_dir_all(&state.config_dir).unwrap();
        let in_config = state.config_dir.join("music.sf2");
        std::fs::write(&in_config, SOUND_FONT_BYTES).unwrap();
        let climbs_out = state
            .config_dir
            .join("..")
            .join("..")
            .join(elsewhere.0.file_name().unwrap())
            .join("music.sf2");
        let store = crate::window::ProfileStore::in_folder(&state.config_dir);
        for (font, expected) in [
            (outside, StatusCode::NOT_FOUND),
            (climbs_out, StatusCode::NOT_FOUND),
            (in_config, StatusCode::OK),
        ] {
            let mut profile = Profile::default();
            profile.sound.midi_sound_font = Some(font.clone());
            store.save_default(&profile);
            let answer = send(state.clone(), get("/v1/soundfont")).await;
            assert_eq!(answer.status(), expected, "{}", font.display());
        }
    }
}
