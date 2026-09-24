//! The copy of the profile an Options window edits, as the reference client's
//! Options keep one: Apply and Okay make it the profile, Cancel lets
//! it go, and Default puts one page back to its defaults. The Classic
//! Options gump and the Modern Options panel both edit through this.

use crate::window::settings::{rows_on, Page, Profile};

/// The profile as the player changes it, until Apply, and as it was when
/// the window took it.
#[derive(Clone, Debug)]
pub struct Draft {
    base: Profile,
    pub edited: Profile,
}

impl Draft {
    /// A draft of the profile as it is now.
    pub fn of(profile: &Profile) -> Self {
        Self {
            base: profile.clone(),
            edited: profile.clone(),
        }
    }

    /// True when the player changed something that is not applied yet.
    pub fn changed(&self) -> bool {
        self.edited != self.base
    }

    /// Puts the options of the draft into the profile. The places, folds
    /// and looks of the windows stay as they are, since they changed while
    /// the Options were open, and so do the open windows, unless the player
    /// changed that list here, and the ignore list, which its own window
    /// changes.
    pub fn apply(&mut self, profile: &mut Profile) {
        let mut next = self.edited.clone();
        next.gumps = std::mem::take(&mut profile.gumps);
        next.anchored = std::mem::take(&mut profile.anchored);
        next.folded = std::mem::take(&mut profile.folded);
        next.looks = std::mem::take(&mut profile.looks);
        if self.edited.interface.open_panels == self.base.interface.open_panels {
            next.interface.open_panels = std::mem::take(&mut profile.interface.open_panels);
        }
        if self.edited.ignore == self.base.ignore {
            next.ignore = std::mem::take(&mut profile.ignore);
        }
        *profile = next;
        self.base = self.edited.clone();
    }

    /// Puts every option of one page back to its default.
    pub fn reset_page(&mut self, page: Page) {
        let defaults = Profile::default();
        for row in rows_on(page) {
            (row.set)(&mut self.edited, (row.get)(&defaults));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::GumpPlace;

    #[test]
    fn apply_keeps_the_places_of_the_windows() {
        let mut profile = Profile::default();
        profile.gumps.insert(
            "status".into(),
            GumpPlace {
                x: 5.0,
                y: 6.0,
                size: None,
                locked: false,
            },
        );
        let mut draft = Draft::of(&Profile::default());
        assert!(!draft.changed());
        draft.edited.general.always_run = true;
        assert!(draft.changed());
        profile.interface.open_panels.push("status".into());
        profile.folded.insert("modern:journal".into());
        profile.looks.insert("buffs".into(), 2);
        draft.apply(&mut profile);
        assert!(!draft.changed(), "an applied draft has no changes left");
        assert!(profile.general.always_run);
        assert!(profile.gumps.contains_key("status"));
        assert!(profile.folded.contains("modern:journal"));
        assert_eq!(profile.looks.get("buffs"), Some(&2));
        assert_eq!(profile.interface.open_panels, vec!["status".to_string()]);
        profile.ignore.add("Ann");
        draft.apply(&mut profile);
        assert_eq!(
            profile.ignore.names,
            vec!["Ann".to_string()],
            "the ignore list window added a name meanwhile"
        );
        draft.edited.interface.open_panels.clear();
        draft.edited.interface.open_panels.push("options".into());
        draft.apply(&mut profile);
        assert_eq!(profile.interface.open_panels, vec!["options".to_string()]);
    }

    #[test]
    fn default_puts_back_only_its_own_page() {
        let mut profile = Profile::default();
        profile.general.always_run = !profile.general.always_run;
        profile.sound.muted = !profile.sound.muted;
        let mut draft = Draft::of(&profile);
        draft.reset_page(Page::General);
        assert_eq!(
            draft.edited.general.always_run,
            Profile::default().general.always_run
        );
        assert_eq!(draft.edited.sound.muted, profile.sound.muted);
    }
}
