//! The action registry: every action a key, a macro step or a button of a
//! game controller can run. Each action has a stable id that the profile
//! keeps, plain words for the Options screen, and the kind of argument it
//! takes.
//!
//! The list covers every macro type of the official client and of
//! the reference client (with each sub type as an argument), and the hotkeys of
//! the session. Two of its types are left out because they do nothing there
//! either: `Aura` and `KillGumpOpen`. `ModifyUpdateRange` is the same as
//! `SetUpdateRange` there, so it is one action here.
//!
//! An action turns into effects (`resolve`): acts of the character, which
//! go to the session through the hand, and window commands, which the
//! window does itself or gives to the UI style that is active.

mod arguments;
pub mod client;
pub mod editor;
pub mod guard;
pub mod journal;
pub mod modern;
mod resolve;
pub mod runner;
pub mod screenshot;
mod select;
pub mod view_range;

pub use arguments::{chosen, ArgumentKind, Direction, GumpKind, Look, SelectKind, ZoomStep};

use crate::window::settings::MacroStep;

/// The groups the Options screen sorts the actions into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    Speech,
    Movement,
    Windows,
    View,
    Combat,
    SpellsAndSkills,
    Targeting,
    Items,
    Macro,
    Pointer,
}

impl Group {
    pub const ALL: [Group; 10] = [
        Group::Speech,
        Group::Movement,
        Group::Windows,
        Group::View,
        Group::Combat,
        Group::SpellsAndSkills,
        Group::Targeting,
        Group::Items,
        Group::Macro,
        Group::Pointer,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Speech => "Speech",
            Group::Movement => "Movement",
            Group::Windows => "Windows",
            Group::View => "View",
            Group::Combat => "Combat",
            Group::SpellsAndSkills => "Spells & skills",
            Group::Targeting => "Targeting",
            Group::Items => "Items",
            Group::Macro => "Macro",
            Group::Pointer => "Mouse",
        }
    }
}

/// One action of the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionSpec {
    pub action: ActionId,
    /// The id the profile keeps. It never changes.
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub argument: ArgumentKind,
}

/// Makes the enum of the actions and the list of their specs from one
/// table, so an action cannot have an id and no spec.
macro_rules! actions {
    ($($variant:ident => $id:literal, $label:literal, $group:ident, $argument:ident;)+) => {
        /// Every action of the registry.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ActionId {
            $($variant),+
        }

        /// Every action, in the order the Options screen shows them.
        pub const ACTIONS: &[ActionSpec] = &[
            $(ActionSpec {
                action: ActionId::$variant,
                id: $id,
                label: $label,
                group: Group::$group,
                argument: ArgumentKind::$argument,
            }),+
        ];
    };
}

actions! {
    // Speech.
    Say => "say", "Say", Speech, Text;
    Emote => "emote", "Emote", Speech, Text;
    Whisper => "whisper", "Whisper", Speech, Text;
    Yell => "yell", "Yell", Speech, Text;
    RazorMacro => "razor_macro", "Assistant macro", Speech, Text;
    Paste => "paste", "Paste into the chat line", Speech, None;
    ToggleChat => "toggle_chat", "Show or hide the chat line", Speech, None;
    Bow => "bow", "Bow", Speech, None;
    Salute => "salute", "Salute", Speech, None;
    // Movement.
    Walk => "walk", "Walk", Movement, Direction;
    WarPeace => "war_peace", "War / peace", Movement, None;
    OpenDoor => "open_door", "Open door", Movement, None;
    AlwaysRun => "always_run", "Always run on / off", Movement, None;
    ClickToRun => "click_to_run", "Click-to-run on / off", Movement, None;
    ToggleFly => "toggle_fly", "Gargoyle fly / land", Movement, None;
    // Windows.
    OpenGump => "open", "Open a window", Windows, Gump;
    CloseGump => "close", "Close a window", Windows, Gump;
    ToggleGump => "toggle", "Open or close a window", Windows, Gump;
    MinimizeGump => "minimize", "Minimize a window", Windows, Gump;
    MaximizeGump => "maximize", "Maximize a window", Windows, Gump;
    CloseAllGumps => "close_gumps", "Close all windows", Windows, None;
    CloseCorpses => "close_corpses", "Close corpses", Windows, None;
    CloseHealthBars => "close_health_bars", "Close all health bars", Windows, None;
    CloseInactiveHealthBars => "close_inactive_health_bars", "Close inactive health bars", Windows, None;
    ToggleBuffGump => "toggle_buff_gump", "Buff window on / off", Windows, None;
    UseCounterSlot => "use_counter_slot", "Use a counter bar slot", Windows, Number;
    SaveDesktop => "save_desktop", "Save the desktop", Windows, None;
    QuitGame => "quit_game", "Quit the game", Windows, None;
    // View.
    Zoom => "zoom", "Zoom", View, Zoom;
    LookAtMouse => "look_at_mouse", "Look toward the mouse (hold)", View, Look;
    Screenshot => "screenshot", "Take a screenshot", View, None;
    CircleOfTransparency => "circle_of_transparency", "Circle of transparency on / off", View, None;
    ToggleRoofs => "toggle_roofs", "Roofs on / off", View, None;
    ToggleTreeStumps => "toggle_tree_stumps", "Trees to stumps on / off", View, None;
    ToggleVegetation => "toggle_vegetation", "Vegetation on / off", View, None;
    ToggleCaveTiles => "toggle_cave_tiles", "Cave tile marks on / off", View, None;
    ToggleNames => "toggle_names", "Names over heads on / off", View, None;
    AllNames => "all_names", "Show all names", View, None;
    ToggleAura => "toggle_aura", "Auras on / off", View, None;
    EnableRangeColor => "range_color_on", "Out of range color on", View, None;
    DisableRangeColor => "range_color_off", "Out of range color off", View, None;
    ToggleRangeColor => "toggle_range_color", "Out of range color on / off", View, None;
    SetViewRange => "view_range_set", "Set the view range", View, Number;
    IncreaseViewRange => "view_range_up", "View range up", View, None;
    DecreaseViewRange => "view_range_down", "View range down", View, None;
    MaxViewRange => "view_range_max", "Largest view range", View, None;
    MinViewRange => "view_range_min", "Smallest view range", View, None;
    DefaultViewRange => "view_range_default", "Default view range", View, None;
    // Combat.
    AttackLast => "attack_last", "Attack last target", Combat, None;
    AttackSelected => "attack_selected", "Attack selected target", Combat, None;
    PrimaryAbility => "primary_ability", "Primary ability", Combat, None;
    SecondaryAbility => "secondary_ability", "Secondary ability", Combat, None;
    ArmDisarm => "arm_disarm", "Arm / disarm", Combat, Hand;
    EquipLastWeapon => "equip_last_weapon", "Equip last weapon", Combat, None;
    BandageSelf => "bandage_self", "Bandage self", Combat, None;
    BandageTarget => "bandage_target", "Bandage target", Combat, None;
    InvokeVirtue => "invoke_virtue", "Invoke virtue", Combat, Virtue;
    TargetSystem => "target_system", "New target system on / off", Combat, None;
    // Spells and skills.
    UseSkill => "use_skill", "Use skill", SpellsAndSkills, Skill;
    LastSkill => "last_skill", "Last skill", SpellsAndSkills, None;
    CastSpell => "cast", "Cast spell", SpellsAndSkills, Spell;
    LastSpell => "last_spell", "Last spell", SpellsAndSkills, None;
    // Targeting.
    LastTarget => "last_target", "Last target", Targeting, None;
    TargetSelf => "target_self", "Target self", Targeting, None;
    WaitForTarget => "wait_for_target", "Wait for target", Targeting, Milliseconds;
    TargetNext => "target_next", "Target next", Targeting, None;
    SelectNext => "select_next", "Select next", Targeting, Select;
    SelectPrevious => "select_previous", "Select previous", Targeting, Select;
    SelectNearest => "select_nearest", "Select nearest", Targeting, Select;
    UseSelected => "use_selected", "Use selected target", Targeting, None;
    TargetSelected => "current_target", "Target selected target", Targeting, None;
    Grab => "grab", "Grab an item", Targeting, None;
    SetGrabBag => "set_grab_bag", "Set the grab bag", Targeting, None;
    // Items.
    LastObject => "last_object", "Use last object", Items, None;
    UseItemInHand => "use_item_in_hand", "Use item in hand", Items, None;
    UsePotion => "use_potion", "Drink potion", Items, Potion;
    UseObject => "use_object", "Use object", Items, Usable;
    // Macro.
    Delay => "delay", "Delay", Macro, Milliseconds;
    RunScript => "run_script", "Run a saved script", Macro, Script;
    Command => "command", "Script command", Macro, Command;
    Hotkey => "hotkey", "Assistant hotkey", Macro, Hotkey;
    // The mouse, for a game controller.
    LeftClick => "left_click", "Left click", Pointer, None;
    RightClick => "right_click", "Right click", Pointer, None;
    DoubleClick => "double_click", "Double click", Pointer, None;
}

impl ActionId {
    pub fn spec(self) -> &'static ActionSpec {
        ACTIONS
            .iter()
            .find(|spec| spec.action == self)
            .expect("every action has a spec")
    }

    /// The action a profile names by its id.
    pub fn from_id(id: &str) -> Option<Self> {
        ACTIONS
            .iter()
            .find(|spec| spec.id == id)
            .map(|spec| spec.action)
    }

    /// An action that lasts while its key is held: it acts on the press
    /// and stops on the release.
    pub fn is_held(self) -> bool {
        matches!(self, ActionId::Walk | ActionId::LookAtMouse)
    }
}

/// The action of a macro step, when the profile names a known one.
pub fn step_action(step: &MacroStep) -> Option<ActionId> {
    ActionId::from_id(&step.action)
}

/// A step of the action with its default argument.
pub fn new_step(action: ActionId) -> MacroStep {
    let spec = action.spec();
    MacroStep::new(spec.id, &spec.argument.default_argument())
}

/// Which way a window command acts on a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GumpOp {
    Open,
    Close,
    Toggle,
    Minimize,
    Maximize,
}

/// An option of the profile an action switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    AlwaysRun,
    ClickToRun,
    CircleOfTransparency,
    HideRoofs,
    TreesToStumps,
    HideVegetation,
    CaveTiles,
    Names,
    Aura,
    OutOfRangeColor,
    NewTargetSystem,
}

/// A change of the view range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeChange {
    Set(u8),
    Up,
    Down,
    Max,
    Min,
    Default,
}

/// How a selection moves through the things in view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectHow {
    Next,
    Previous,
    Nearest,
}

/// What the next click on a thing does, while the window waits for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalAim {
    /// The item clicked goes into the grab bag.
    Grab,
    /// The container clicked becomes the grab bag.
    SetGrabBag,
    /// The thing clicked is kept for the window to read, as the eyedropper
    /// of the color picker takes a hue.
    PickThing,
    /// The player clicked is kept for the ignore list to read.
    IgnorePlayer,
}

/// A mouse click a controller button makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerClick {
    Left,
    Right,
    Double,
}

/// Something the window does, not the character. The window does the
/// shared ones itself (options, zoom, screenshots, the selection) and gives
/// the others (`is_for_style`) to the UI style that is active.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowCommand {
    Gump(GumpOp, GumpKind),
    CloseAllGumps,
    CloseCorpses,
    CloseHealthBars { inactive_only: bool },
    UseCounterSlot(u8),
    ToggleChat,
    PasteToChat,
    ToggleOption(Switch),
    SetOption(Switch, bool),
    Zoom(ZoomStep),
    Screenshot,
    SaveDesktop,
    QuitGame,
    ViewRange(RangeChange),
    Select(SelectHow, SelectKind),
    Aim(LocalAim),
    Click(PointerClick),
}

impl WindowCommand {
    /// A command only the UI style can do: it draws the windows and the
    /// chat line.
    pub fn is_for_style(&self) -> bool {
        matches!(
            self,
            WindowCommand::Gump(..)
                | WindowCommand::CloseAllGumps
                | WindowCommand::CloseCorpses
                | WindowCommand::CloseHealthBars { .. }
                | WindowCommand::UseCounterSlot(_)
                | WindowCommand::ToggleChat
                | WindowCommand::PasteToChat
                | WindowCommand::QuitGame
        )
    }
}

/// A UI style: it opens, closes and moves its own windows. The Classic and
/// the Modern style each have one.
pub trait StyleWindows {
    /// Does one command of `WindowCommand::is_for_style`. False when this
    /// style has no such window; the window then tells the player so.
    fn command(&mut self, command: &WindowCommand) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_id_is_unique_and_names_its_own_action() {
        let mut ids = HashSet::new();
        let mut actions = HashSet::new();
        for spec in ACTIONS {
            assert!(ids.insert(spec.id), "{} is used twice", spec.id);
            assert!(actions.insert(spec.action), "{:?} twice", spec.action);
            assert_eq!(ActionId::from_id(spec.id), Some(spec.action));
            assert_eq!(spec.action.spec().id, spec.id);
            assert!(!spec.label.is_empty());
        }
        assert_eq!(ActionId::from_id("no such action"), None);
    }

    #[test]
    fn every_group_has_actions() {
        for group in Group::ALL {
            assert!(ACTIONS.iter().any(|spec| spec.group == group), "{group:?}");
        }
    }

    #[test]
    fn a_new_step_names_its_action_and_a_default_argument() {
        let step = new_step(ActionId::CastSpell);
        assert_eq!(step_action(&step), Some(ActionId::CastSpell));
        assert!(!step.argument.is_empty());
        assert!(new_step(ActionId::WarPeace).argument.is_empty());
    }

    #[test]
    fn each_style_quits_its_own_way() {
        assert!(WindowCommand::QuitGame.is_for_style());
        assert!(!WindowCommand::Screenshot.is_for_style());
    }
}
