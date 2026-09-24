//! The options that are one choice from a short list. Each list has the
//! words a player reads, in the order the Options screen shows them.

use eframe::egui::Modifiers;
use serde::{Deserialize, Serialize};

/// A choice from a fixed list. The index is the place in `LABELS`.
pub trait Choice: Copy + 'static {
    /// The words for each choice, in order.
    const LABELS: &'static [&'static str];
    fn index(self) -> usize;
    /// The choice at `index`. An index past the end gives the first choice.
    fn from_index(index: usize) -> Self;
}

/// Makes one list of choices and its words.
macro_rules! choices {
    ($(#[$doc:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $($variant),+
        }

        impl Choice for $name {
            const LABELS: &'static [&'static str] = &[$($label),+];

            fn index(self) -> usize {
                self as usize
            }

            fn from_index(index: usize) -> Self {
                const ALL: &[$name] = &[$($name::$variant),+];
                ALL.get(index).copied().unwrap_or(ALL[0])
            }
        }
    };
}

/// Other parts of the window make their own lists of choices with it.
pub(crate) use choices;

choices! {
    /// The pages of the Options screen.
    Page {
        General => "General",
        Sound => "Sound",
        Video => "Video",
        Macros => "Macros",
        Tooltip => "Tooltip",
        Fonts => "Fonts",
        Speech => "Speech",
        CombatSpells => "Combat & Spells",
        Counters => "Counters",
        InfoBar => "Info Bar",
        Containers => "Containers",
        Experimental => "Experimental",
        IgnoreList => "Ignore List",
        Interface => "Interface",
        Nameplates => "Nameplates",
        Journal => "Journal",
        WorldMap => "World Map",
        Agents => "Agents",
    }
}

choices! {
    /// How the play window looks: the official client's gumps, or the
    /// glass panels of the command deck.
    UiStyle {
        Classic => "Classic",
        Modern => "Modern",
    }
}

choices! {
    /// How the play window sits on the screen.
    WindowMode {
        Windowed => "Windowed",
        Borderless => "Borderless",
        Fullscreen => "Full screen",
    }
}

choices! {
    /// When a corpse near the character opens by itself.
    CorpseOpenRule {
        Always => "Always",
        UnlessTargeting => "Not while targeting",
        UnlessHidden => "Not while hidden",
        UnlessTargetingOrHidden => "Not while targeting or hidden",
    }
}

choices! {
    /// How the hit points of a mobile show over it.
    HpStyle {
        Percentage => "Percentage",
        Line => "Line",
        Both => "Both",
    }
}

choices! {
    /// When the hit points of a mobile show over it.
    HpShowWhen {
        Always => "Always",
        BelowFull => "Less than 100%",
        Smart => "Smart",
    }
}

choices! {
    /// When a ring of color shows under the feet of mobiles.
    AuraRule {
        Never => "None",
        WarMode => "War mode",
        CtrlShift => "Ctrl+Shift",
        Always => "Always",
    }
}

choices! {
    /// When a health bar gump closes by itself.
    CloseHealthBar {
        Never => "None",
        OutOfRange => "Mobile out of range",
        Dead => "Mobile is dead",
    }
}

choices! {
    /// How a corpse shows its loot.
    GridLoot {
        Off => "None",
        GridOnly => "Grid loot only",
        Both => "Both",
    }
}

choices! {
    /// The edge of the circle of transparency.
    CircleStyle {
        Full => "Full",
        Gradient => "Gradient",
    }
}

choices! {
    /// A key held with the mouse.
    ModifierKey {
        None => "None",
        Ctrl => "Ctrl",
        Shift => "Shift",
        Alt => "Alt",
    }
}

impl ModifierKey {
    /// True when this key is down. None is never down.
    pub fn is_held(self, modifiers: Modifiers) -> bool {
        match self {
            Self::None => false,
            Self::Ctrl => modifiers.ctrl,
            Self::Shift => modifiers.shift,
            Self::Alt => modifiers.alt,
        }
    }
}

choices! {
    /// How fire, poison, energy and paralyze fields draw.
    FieldStyle {
        Normal => "Normal fields",
        Static => "Static fields",
        Tile => "Tile fields",
    }
}

choices! {
    /// What the chosen light level means.
    LightLevelRule {
        Absolute => "Absolute",
        Minimum => "Minimum",
    }
}

choices! {
    /// The kind of UO font that takes the place of every game font.
    GameFontKind {
        Ascii => "ASCII",
        Unicode => "Unicode",
    }
}

choices! {
    /// How the info bar shows that a value is low.
    InfoBarHighlight {
        TextColor => "Text color",
        ColoredBars => "Colored bars",
    }
}

choices! {
    /// The value one info bar item shows.
    InfoBarData {
        HitPoints => "Hits",
        Mana => "Mana",
        Stamina => "Stamina",
        Weight => "Weight",
        Followers => "Followers",
        Gold => "Gold",
        Damage => "Damage",
        Armor => "Armor",
        Luck => "Luck",
        FireResist => "Fire resist",
        ColdResist => "Cold resist",
        PoisonResist => "Poison resist",
        EnergyResist => "Energy resist",
        LowerReagentCost => "Lower reagent cost",
        SpellDamage => "Spell damage increase",
        FasterCasting => "Faster casting",
        FasterCastRecovery => "Faster cast recovery",
        HitChance => "Hit chance increase",
        DefenseChance => "Defense chance increase",
        LowerManaCost => "Lower mana cost",
        DamageIncrease => "Damage increase",
        SwingSpeed => "Swing speed increase",
        StatsCap => "Stats cap",
        Name => "Name",
        TithingPoints => "Tithing points",
    }
}

choices! {
    /// The art of the character's own backpack.
    BackpackStyle {
        Default => "Default",
        Suede => "Suede",
        PolarBear => "Polar bear",
        GhoulSkin => "Ghoul skin",
    }
}

choices! {
    /// Where a container gump opens when its place is overridden.
    ContainerPlace {
        NearContainer => "Near container position",
        TopRight => "Top right",
        LastDragged => "Last dragged position",
        RememberEach => "Remember every container",
    }
}

choices! {
    /// What a search in a grid container does.
    GridSearch {
        Filter => "Hide items that do not match",
        Highlight => "Highlight items that match",
    }
}

choices! {
    /// Which things get a nameplate.
    NameplateFilter {
        All => "All",
        Mobiles => "Mobiles",
        Items => "Items",
        Corpses => "Corpses",
        MobilesAndCorpses => "Mobiles and corpses",
    }
}

choices! {
    /// The kinds of journal lines a journal tab can show.
    JournalKind {
        Speech => "Speech",
        Emote => "Emote",
        Whisper => "Whisper",
        Yell => "Yell",
        System => "System",
        Label => "Label",
        Spell => "Spell",
        Party => "Party",
        Guild => "Guild",
        Alliance => "Alliance",
        Chat => "Chat",
    }
}

choices! {
    /// Whose journal lines start a cooldown bar.
    CooldownSource {
        Anyone => "Anyone",
        Myself => "Only me",
        Others => "Only others",
        System => "Only the shard",
    }
}

choices! {
    /// How the title of the window shows the vitals of the character.
    TitleStats {
        Numbers => "Numbers",
        Percent => "Percent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_choice_comes_back_from_its_index_and_a_bad_index_gives_the_first() {
        for (index, _) in InfoBarData::LABELS.iter().enumerate() {
            assert_eq!(InfoBarData::from_index(index).index(), index);
        }
        assert_eq!(
            WindowMode::from_index(WindowMode::LABELS.len()),
            WindowMode::Windowed
        );
    }
}
