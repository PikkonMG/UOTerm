//! The panel launcher of the Modern style: the panels it opens and
//! closes, whether each shows, and the close of every panel of its own.
//! The ids keep each panel open and its place in the profile.

use crate::model::agents::AgentPanel;
use crate::model::places;
use crate::settings::Profile;

pub const RADAR_ID: &str = "modern:radar";
pub const JOURNAL_ID: &str = "modern:journal";
pub const DURABILITY_ID: &str = "modern:durability";
pub const DPS_ID: &str = "modern:dps";
pub const AGENTS_ID: &str = "modern:agents";
pub const IGNORE_ID: &str = "modern:ignore";
pub const NET_STATS_ID: &str = "modern:net_stats";
pub const DEBUG_ID: &str = "modern:debug";

/// The words of the launcher, and of the button of the control bar that
/// opens it.
pub const WORDS_LAUNCHER: &str = "Panels";
const WORDS_LOOT: &str = "Loot";
const WORDS_DURABILITY: &str = "Durability";
const WORDS_DAMAGE: &str = "Damage";
const WORDS_AGENTS: &str = "Agents";
const WORDS_COUNTERS: &str = "Counters";
const WORDS_INFO: &str = "Info bar";
const WORDS_RADAR: &str = "Radar";
const WORDS_JOURNAL: &str = "Journal";
const WORDS_BUFFS: &str = "Buffs";
const WORDS_NET_STATS: &str = "Network";
const WORDS_DEBUG: &str = "Debug";

/// The panels a close of every window closes, by the ids that keep them
/// open.
const CLOSING_PANELS: [&str; 6] = [
    DURABILITY_ID,
    DPS_ID,
    AGENTS_ID,
    IGNORE_ID,
    NET_STATS_ID,
    DEBUG_ID,
];

/// A button of the launcher: the panel it opens and closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    Loot,
    Durability,
    Damage,
    Agents,
    Counters,
    InfoBar,
    Radar,
    Journal,
    Buffs,
    NetStats,
    Debug,
}

/// The buttons of the launcher in order, each with its words.
pub const LAUNCHES: [(Launch, &str); 11] = [
    (Launch::Radar, WORDS_RADAR),
    (Launch::Journal, WORDS_JOURNAL),
    (Launch::Loot, WORDS_LOOT),
    (Launch::Durability, WORDS_DURABILITY),
    (Launch::Damage, WORDS_DAMAGE),
    (Launch::Agents, WORDS_AGENTS),
    (Launch::Counters, WORDS_COUNTERS),
    (Launch::InfoBar, WORDS_INFO),
    (Launch::Buffs, WORDS_BUFFS),
    (Launch::NetStats, WORDS_NET_STATS),
    (Launch::Debug, WORDS_DEBUG),
];

impl Launch {
    /// True when the panel shows.
    pub fn shows(self, profile: &Profile) -> bool {
        match self {
            Self::Loot => profile.interface.nearby_loot_window,
            Self::Durability => places::is_open(profile, DURABILITY_ID),
            Self::Damage => places::is_open(profile, DPS_ID),
            Self::Agents => places::is_open(profile, AGENTS_ID),
            Self::Counters => profile.counters.enabled,
            Self::InfoBar => profile.info_bar.enabled,
            Self::Radar => !places::is_shut(profile, RADAR_ID),
            Self::Journal => !places::is_shut(profile, JOURNAL_ID),
            Self::Buffs => profile.combat.improved_buff_bar,
            Self::NetStats => places::is_open(profile, NET_STATS_ID),
            Self::Debug => places::is_open(profile, DEBUG_ID),
        }
    }

    /// Shows the panel, or hides it.
    pub fn set(self, profile: &mut Profile, shows: bool) {
        match self {
            Self::Loot => profile.interface.nearby_loot_window = shows,
            Self::Durability => places::set_open(profile, DURABILITY_ID, shows),
            Self::Damage => places::set_open(profile, DPS_ID, shows),
            Self::Agents => places::set_open(profile, AGENTS_ID, shows),
            Self::Counters => profile.counters.enabled = shows,
            Self::InfoBar => profile.info_bar.enabled = shows,
            Self::Radar => places::set_shut(profile, RADAR_ID, !shows),
            Self::Journal => places::set_shut(profile, JOURNAL_ID, !shows),
            Self::Buffs => profile.combat.improved_buff_bar = shows,
            Self::NetStats => places::set_open(profile, NET_STATS_ID, shows),
            Self::Debug => places::set_open(profile, DEBUG_ID, shows),
        }
    }
}

/// Closes every panel of its own the profile keeps open: the loot,
/// durability, damage and agent windows, the network statistics and the
/// debug window. The journal, the radar and the bars the pages switch on
/// stay, as in the classic client.
pub fn close_all(profile: &mut Profile) {
    profile.interface.nearby_loot_window = false;
    for id in CLOSING_PANELS {
        places::set_open(profile, id, false);
    }
    for agent in AgentPanel::ALL {
        places::set_open(profile, &agent.place_id(), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_launch_turns_its_panel_and_a_close_of_all_keeps_the_journal() {
        let mut profile = Profile::default();
        for (launch, _) in LAUNCHES {
            let shows = launch.shows(&profile);
            launch.set(&mut profile, !shows);
            assert_eq!(launch.shows(&profile), !shows, "{launch:?}");
        }
        Launch::Journal.set(&mut profile, true);
        Launch::Loot.set(&mut profile, true);
        Launch::Agents.set(&mut profile, true);
        places::set_open(&mut profile, &AgentPanel::Dress.place_id(), true);
        close_all(&mut profile);
        assert!(!Launch::Loot.shows(&profile) && !Launch::Agents.shows(&profile));
        assert!(!places::is_open(&profile, &AgentPanel::Dress.place_id()));
        assert!(Launch::Journal.shows(&profile));
    }
}
