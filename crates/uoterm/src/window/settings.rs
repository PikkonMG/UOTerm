//! The one settings model of the play window: the profile. It holds every
//! option a player sets, page by page as the Options screen shows them, and
//! where each floating gump was left. The window keeps one global default
//! profile and one for each character of each shard (see `store`).
//!
//! The option table (`table`) describes each option for the Options screen:
//! its page, section, label and control, and how to read and write it.

mod choices;
mod keys;
mod pages;
mod store;
mod table;

pub use choices::*;
pub use keys::{KeyBinding, KeyChord, MacroStep, PadChord};
pub use pages::*;
pub use store::{shard_address, ProfileHome};
pub use table::{rows_on, OptionKind, OptionRow, OptionValue};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Every option of one player, and the places of his gumps.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub general: GeneralOptions,
    pub sound: SoundOptions,
    pub video: VideoOptions,
    pub macros: MacroOptions,
    pub tooltip: TooltipOptions,
    pub fonts: FontOptions,
    pub speech: SpeechOptions,
    pub combat: CombatOptions,
    pub counters: CounterOptions,
    pub info_bar: InfoBarOptions,
    pub containers: ContainerOptions,
    pub experimental: ExperimentalOptions,
    pub ignore: IgnoreOptions,
    pub interface: InterfaceOptions,
    pub nameplates: NameplateOptions,
    pub journal: JournalOptions,
    pub world_map: WorldMapOptions,
    pub agents: AgentPanelOptions,
    /// Where each floating gump was left, by the id of its kind, such as
    /// `paperdoll`, `status` or `container:3C`.
    pub gumps: BTreeMap<String, GumpPlace>,
    /// The classic gumps joined to others, by the key of their place, and
    /// where each sits in its group.
    pub anchored: BTreeMap<String, AnchorCell>,
    /// The classic gumps folded small, such as the menu bar, by the key of
    /// their place.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub folded: BTreeSet<String>,
    /// The look the button of a classic gump turned to, such as the
    /// background of the buff gump, by the key of its place. A gump on its
    /// first look has none.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub looks: BTreeMap<String, u8>,
    /// The groups of the skills gump as the player made them, in order.
    /// Empty keeps the groups of the client files.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skill_groups: Vec<SkillGroupSet>,
}

/// One group of the skills gump: its name, its skills by number, and
/// whether it shows them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGroupSet {
    pub name: String,
    pub skills: Vec<u16>,
    #[serde(default)]
    pub open: bool,
}

/// Where a joined gump sits in its anchor group: the group, and its cell
/// in the grid of the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorCell {
    pub group: u32,
    pub column: i32,
    pub row: i32,
}

/// Where a floating gump was left, in points of the window.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GumpPlace {
    pub x: f32,
    pub y: f32,
    /// The width and the height, for a gump the player can resize.
    #[serde(default)]
    pub size: Option<(f32, f32)>,
    /// A locked gump does not move or close by a click.
    #[serde(default)]
    pub locked: bool,
}

/// The kinds of sound that have a volume of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundKind {
    Music,
    Effects,
    Footsteps,
}

impl SoundOptions {
    /// How loud one kind plays: its own volume under the master volume.
    /// A kind that is off, or silence, gives 0.
    pub fn volume(&self, kind: SoundKind) -> f32 {
        let (on, own) = match kind {
            SoundKind::Music => (self.music_on, self.music_volume),
            SoundKind::Effects => (self.sound_on, self.sound_volume),
            SoundKind::Footsteps => (self.footsteps_on, self.footsteps_volume),
        };
        if self.muted || !on {
            return 0.0;
        }
        (own * self.master_volume).clamp(0.0, 1.0)
    }

    /// The player does not want to hear this sound effect.
    pub fn filters_sound(&self, sound: u16) -> bool {
        self.sound_filter.contains(&sound)
    }

    /// The player does not want to hear this music track.
    pub fn filters_music(&self, music: u16) -> bool {
        self.music_filter.contains(&music)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_is_under_the_master_and_mute_or_off_silences_it() {
        let mut sound = SoundOptions {
            master_volume: 0.5,
            music_volume: 0.4,
            ..SoundOptions::default()
        };
        assert!((sound.volume(SoundKind::Music) - 0.2).abs() < f32::EPSILON);
        sound.footsteps_on = false;
        assert_eq!(sound.volume(SoundKind::Footsteps), 0.0);
        assert!(sound.volume(SoundKind::Effects) > 0.0);
        sound.muted = true;
        assert_eq!(sound.volume(SoundKind::Effects), 0.0);
    }

    #[test]
    fn filtered_sounds_and_music_are_known() {
        let sound = SoundOptions {
            sound_filter: vec![0x0123],
            music_filter: vec![7],
            ..SoundOptions::default()
        };
        assert!(sound.filters_sound(0x0123) && !sound.filters_sound(0x0124));
        assert!(sound.filters_music(7) && !sound.filters_music(8));
    }

    #[test]
    fn hidden_layers_hide_on_the_character_or_on_every_mobile() {
        const CLOAK: u8 = 0x14;
        const HELMET: u8 = 0x06;
        let mut general = GeneralOptions::default();
        general.set_layer_hidden(CLOAK, true);
        general.set_layer_hidden(HELMET, true);
        general.set_layer_hidden(CLOAK, true);
        assert_eq!(general.hidden_layers, [HELMET, CLOAK]);
        assert!(!general.hides_layer(CLOAK, true), "the system is off");
        general.hidden_layers_enabled = true;
        assert!(general.hides_layer(CLOAK, true));
        assert!(!general.hides_layer(CLOAK, false), "only on the character");
        general.hide_layers_for_self = false;
        assert!(general.hides_layer(CLOAK, false));
        general.set_layer_hidden(CLOAK, false);
        assert!(!general.hides_layer(CLOAK, true));
        assert_eq!(general.hidden_layers, [HELMET]);
    }

    #[test]
    fn the_defaults_are_the_modern_look_and_the_old_volumes() {
        let profile = Profile::default();
        assert_eq!(profile.interface.ui_style, UiStyle::Modern);
        assert_eq!(profile.sound.master_volume, DEFAULT_MASTER_VOLUME);
        assert_eq!(profile.sound.music_volume, DEFAULT_MUSIC_VOLUME);
        assert_eq!(profile.video.fps, DEFAULT_FPS);
        assert!(profile.gumps.is_empty() && profile.macros.key_bindings.is_empty());
        let text = toml::to_string(&profile).unwrap();
        assert_eq!(toml::from_str::<Profile>(&text).unwrap(), profile);
        assert_eq!(toml::from_str::<Profile>("").unwrap(), profile);
    }
}
