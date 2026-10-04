//! What each action does: the acts it sends and the window commands it
//! gives, for the argument of its step.

use super::arguments::{
    chosen, Direction, GumpKind, Hand, Look, SelectKind, UsableObject, Virtue, ZoomStep,
    DEFAULT_TARGET_WAIT_MS,
};
use super::{
    ActionId, GumpOp, LocalAim, PointerClick, RangeChange, SelectHow, Switch, WindowCommand,
};
use crate::view::WatchFrame;
use crate::window::control::{quoted, Act, Channel};
use crate::window::keys::chat::channel_hue;
use crate::window::macros_ui::play_line;
use crate::window::settings::{Choice, Profile, SpeechOptions};
use std::time::Duration;

/// Hotkeys of the session that some actions press.
const HOTKEY_FLY: &str = "Fly On/Off";
const HOTKEY_PRIMARY: &str = "Primary Ability";
const HOTKEY_SECONDARY: &str = "Secondary Ability";
const HOTKEY_LEFT_HAND: &str = "Toggle Left Hand";
const HOTKEY_RIGHT_HAND: &str = "Toggle Right Hand";
const HOTKEY_BANDAGE_SELF: &str = "Bandage Self";
const HOTKEY_BANDAGE_LAST: &str = "Bandage Last";
const HOTKEY_LAST_SKILL: &str = "Last Skill";
const HOTKEY_LAST_SPELL: &str = "Last Spell";
const HOTKEY_TARGET_LAST: &str = "Target Last";
const HOTKEY_TARGET_SELF: &str = "Target Self";
const HOTKEY_LAST_OBJECT: &str = "Use Last Item";
const HOTKEY_NAMES_MOBILES: &str = "Show Names Mobiles";
const HOTKEY_NAMES_CORPSES: &str = "Show Names Corpses";
const HOTKEY_APPLE: &str = "Enchanted Apple";
const HOTKEY_ROSE: &str = "Rose Of Trinsic";
const HOTKEY_ORANGE_PETALS: &str = "Orange Petals";
const HOTKEY_SMOKE_BOMB: &str = "Smoke Bomb";
const HOTKEY_HEALING_STONE: &str = "Healing Stone";
const HOTKEY_SPELL_STONE: &str = "Spell Stone";
/// A potion hotkey is this and the potion name.
const HOTKEY_POTION: &str = "Potion ";

const COMMAND_OPEN_DOOR: &str = "opendoor";
const COMMAND_GUILD: &str = "guildbutton";
const COMMAND_QUESTS: &str = "questsbutton";
/// Uses what the character holds: the one-handed layer first, then the
/// two-handed one, as the official client does.
const COMMAND_USE_IN_HAND: &str =
    "if findlayer 'self' 1\nuseobject 'found'\nelseif findlayer 'self' 2\nuseobject 'found'\nendif";
const EMOTE_BOW: &str = "bow";
const EMOTE_SALUTE: &str = "salute";
/// An assistant macro is said with this in front, as the official client
/// does.
const RAZOR_MACRO_PREFIX: &str = ">macro ";
/// Words in the name of a trapped box or chest.
const TRAPPED_WORD: &str = "trap";
/// Below this stamina a character cannot run.
const STAMINA_TO_RUN: u16 = 1;

const NOTE_NO_LAST_TARGET: &str = "There is no last target.";
const NOTE_NO_SELECTION: &str = "No target is selected.";
const NOTE_NO_BACKPACK: &str = "The shard has not told which item is the backpack.";
const NOTE_NO_TRAPPED_BOX: &str = "There is no trapped box in an open container.";

/// What an action needs to know of the game and of the window.
pub struct Context<'a> {
    pub frame: &'a WatchFrame,
    pub profile: &'a Profile,
    /// The target the window selected with the select actions.
    pub selected: Option<u32>,
}

/// Something a step makes wait before the next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    For(Duration),
    /// Until a target cursor opens, or at most this long.
    Target(Duration),
}

/// One effect of an action, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Act(Act),
    Window(WindowCommand),
    Wait(Wait),
    /// A line for the journal: why the action did nothing.
    Note(String),
}

fn hotkey(name: &str) -> Effect {
    Effect::Act(Act::Hotkey(name.to_string()))
}

fn command(line: String) -> Effect {
    Effect::Act(Act::Command(line))
}

fn window(command: WindowCommand) -> Effect {
    Effect::Window(command)
}

fn note(words: impl Into<String>) -> Effect {
    Effect::Note(words.into())
}

/// Words that name no choice of the list.
fn not_a_choice(argument: &str) -> Effect {
    note(format!(
        "\"{argument}\" is not one of the choices of this action."
    ))
}

/// A choice of a list, or the note that the argument names none.
fn with_choice<T: Choice>(argument: &str, make: impl FnOnce(T) -> Vec<Effect>) -> Vec<Effect> {
    chosen::<T>(argument).map_or_else(|| vec![not_a_choice(argument)], make)
}

/// A number, or the note that the argument is none.
fn with_number<T: std::str::FromStr>(
    argument: &str,
    make: impl FnOnce(T) -> Vec<Effect>,
) -> Vec<Effect> {
    argument.trim().parse().map_or_else(
        |_| vec![note(format!("\"{argument}\" is not a number."))],
        make,
    )
}

/// Milliseconds, with the default for an empty argument.
fn milliseconds(argument: &str, default_ms: u64) -> Option<Duration> {
    let words = argument.trim();
    if words.is_empty() {
        return Some(Duration::from_millis(default_ms));
    }
    words.parse().ok().map(Duration::from_millis)
}

/// Whether a step opens a closed door ahead: the General page's auto open
/// doors, while the character lives, as the official client decides.
pub fn opens_doors(frame: &WatchFrame, profile: &Profile) -> bool {
    profile.general.auto_open_doors && !frame.dead
}

/// Whether a step runs: always run, but not while hidden when that option
/// says so, and never out of breath, as the official client decides.
pub fn runs(frame: &WatchFrame, profile: &Profile) -> bool {
    let general = &profile.general;
    general.always_run
        && !(frame.hidden && general.always_run_unless_hidden)
        && (frame.stam > STAMINA_TO_RUN || frame.dead)
}

/// Words on a channel, in the hue the Speech page gives it.
fn say(channel: Option<Channel>, text: &str, speech: &SpeechOptions) -> Vec<Effect> {
    let (text, hue) = (text.to_string(), channel_hue(speech, channel));
    vec![Effect::Act(match channel {
        Some(channel) => Act::Speak { channel, text, hue },
        None => Act::Say { text, hue },
    })]
}

fn gump(op: GumpOp, kind: GumpKind, frame: &WatchFrame) -> Vec<Effect> {
    let opens = matches!(op, GumpOp::Open | GumpOp::Toggle);
    match kind {
        // The shard draws these. Opening one is a request to the shard.
        GumpKind::Guild if opens => vec![command(COMMAND_GUILD.to_string())],
        GumpKind::QuestLog if opens => vec![command(COMMAND_QUESTS.to_string())],
        GumpKind::Guild | GumpKind::QuestLog => Vec::new(),
        // The backpack opens as a container the shard sends.
        GumpKind::Backpack if op == GumpOp::Open => match frame.backpack() {
            Some(bag) => vec![
                Effect::Act(Act::Use(bag)),
                window(WindowCommand::Gump(op, kind)),
            ],
            None => vec![note(NOTE_NO_BACKPACK)],
        },
        _ => vec![window(WindowCommand::Gump(op, kind))],
    }
}

fn use_object(object: UsableObject, frame: &WatchFrame) -> Vec<Effect> {
    let potion = |name: &str| vec![hotkey(&format!("{HOTKEY_POTION}{name}"))];
    match object {
        UsableObject::BestHealPotion => potion("heal"),
        UsableObject::BestCurePotion => potion("cure"),
        UsableObject::BestRefreshPotion => potion("refresh"),
        UsableObject::BestStrengthPotion => potion("strength"),
        UsableObject::BestAgilityPotion => potion("agility"),
        UsableObject::BestExplosionPotion => potion("explosion"),
        UsableObject::BestConflagrationPotion => potion("conflagration"),
        UsableObject::EnchantedApple => vec![hotkey(HOTKEY_APPLE)],
        UsableObject::PetalsOfTrinsic => vec![hotkey(HOTKEY_ROSE)],
        UsableObject::OrangePetals => vec![hotkey(HOTKEY_ORANGE_PETALS)],
        UsableObject::SmokeBomb => vec![hotkey(HOTKEY_SMOKE_BOMB)],
        UsableObject::HealingStone => vec![hotkey(HOTKEY_HEALING_STONE)],
        UsableObject::SpellStone => vec![hotkey(HOTKEY_SPELL_STONE)],
        UsableObject::TrappedBox => frame
            .containers
            .iter()
            .flat_map(|container| &container.items)
            .find(|item| item.name.to_ascii_lowercase().contains(TRAPPED_WORD))
            .map_or_else(
                || vec![note(NOTE_NO_TRAPPED_BOX)],
                |item| vec![Effect::Act(Act::Use(item.serial))],
            ),
    }
}

/// Acts on the selected target, or says there is none.
fn on_selected(context: &Context<'_>, make: impl FnOnce(u32) -> Vec<Effect>) -> Vec<Effect> {
    context
        .selected
        .map_or_else(|| vec![note(NOTE_NO_SELECTION)], make)
}

/// The effects of one action with its argument.
pub fn resolve(action: ActionId, argument: &str, context: &Context<'_>) -> Vec<Effect> {
    let frame = context.frame;
    let speech = &context.profile.speech;
    let words = argument.trim();
    let select = |how| {
        with_choice::<SelectKind>(words, |kind| vec![window(WindowCommand::Select(how, kind))])
    };
    let toggle = |switch| vec![window(WindowCommand::ToggleOption(switch))];
    let range = |change| vec![window(WindowCommand::ViewRange(change))];
    match action {
        ActionId::Say => say(None, words, speech),
        ActionId::Emote => say(Some(Channel::Emote), words, speech),
        ActionId::Whisper => say(Some(Channel::Whisper), words, speech),
        ActionId::Yell => say(Some(Channel::Yell), words, speech),
        ActionId::RazorMacro => say(None, &format!("{RAZOR_MACRO_PREFIX}{words}"), speech),
        ActionId::Paste => vec![window(WindowCommand::PasteToChat)],
        ActionId::ToggleChat => vec![window(WindowCommand::ToggleChat)],
        ActionId::Bow => vec![command(format!("emoteaction {}", quoted(EMOTE_BOW)))],
        ActionId::Salute => vec![command(format!("emoteaction {}", quoted(EMOTE_SALUTE)))],
        ActionId::Walk => with_choice::<Direction>(words, |way| {
            vec![Effect::Act(Act::Step {
                direction: way.way(),
                run: runs(frame, context.profile),
                open_doors: opens_doors(frame, context.profile),
            })]
        }),
        ActionId::WarPeace => vec![Effect::Act(Act::War(!frame.war))],
        ActionId::OpenDoor => vec![command(COMMAND_OPEN_DOOR.to_string())],
        ActionId::AlwaysRun => toggle(Switch::AlwaysRun),
        ActionId::ClickToRun => toggle(Switch::ClickToRun),
        ActionId::ToggleFly => vec![hotkey(HOTKEY_FLY)],
        ActionId::OpenGump => with_choice(words, |kind| gump(GumpOp::Open, kind, frame)),
        ActionId::CloseGump => with_choice(words, |kind| gump(GumpOp::Close, kind, frame)),
        ActionId::ToggleGump => with_choice(words, |kind| gump(GumpOp::Toggle, kind, frame)),
        ActionId::MinimizeGump => with_choice(words, |kind| gump(GumpOp::Minimize, kind, frame)),
        ActionId::MaximizeGump => with_choice(words, |kind| gump(GumpOp::Maximize, kind, frame)),
        ActionId::CloseAllGumps => vec![window(WindowCommand::CloseAllGumps)],
        ActionId::CloseCorpses => vec![window(WindowCommand::CloseCorpses)],
        ActionId::CloseHealthBars => vec![window(WindowCommand::CloseHealthBars {
            inactive_only: false,
        })],
        ActionId::CloseInactiveHealthBars => vec![window(WindowCommand::CloseHealthBars {
            inactive_only: true,
        })],
        ActionId::ToggleBuffGump => gump(GumpOp::Toggle, GumpKind::Buffs, frame),
        ActionId::UseCounterSlot => with_number(words, |slot| {
            vec![window(WindowCommand::UseCounterSlot(slot))]
        }),
        ActionId::SaveDesktop => vec![window(WindowCommand::SaveDesktop)],
        ActionId::QuitGame => vec![window(WindowCommand::QuitGame)],
        ActionId::Zoom => {
            with_choice::<ZoomStep>(words, |step| vec![window(WindowCommand::Zoom(step))])
        }
        // The camera looks while the key is held: the keys do it, and a
        // step of a longer macro does nothing, as in the official client.
        ActionId::LookAtMouse => with_choice::<Look>(words, |_| Vec::new()),
        ActionId::Screenshot => vec![window(WindowCommand::Screenshot)],
        ActionId::CircleOfTransparency => toggle(Switch::CircleOfTransparency),
        ActionId::ToggleRoofs => toggle(Switch::HideRoofs),
        ActionId::ToggleTreeStumps => toggle(Switch::TreesToStumps),
        ActionId::ToggleVegetation => toggle(Switch::HideVegetation),
        ActionId::ToggleCaveTiles => toggle(Switch::CaveTiles),
        ActionId::ToggleNames => toggle(Switch::Names),
        ActionId::AllNames => vec![hotkey(HOTKEY_NAMES_MOBILES), hotkey(HOTKEY_NAMES_CORPSES)],
        ActionId::ToggleAura => toggle(Switch::Aura),
        ActionId::EnableRangeColor => {
            vec![window(WindowCommand::SetOption(
                Switch::OutOfRangeColor,
                true,
            ))]
        }
        ActionId::DisableRangeColor => {
            vec![window(WindowCommand::SetOption(
                Switch::OutOfRangeColor,
                false,
            ))]
        }
        ActionId::ToggleRangeColor => toggle(Switch::OutOfRangeColor),
        ActionId::SetViewRange => with_number(words, |tiles| range(RangeChange::Set(tiles))),
        ActionId::IncreaseViewRange => range(RangeChange::Up),
        ActionId::DecreaseViewRange => range(RangeChange::Down),
        ActionId::MaxViewRange => range(RangeChange::Max),
        ActionId::MinViewRange => range(RangeChange::Min),
        ActionId::DefaultViewRange => range(RangeChange::Default),
        ActionId::AttackLast => frame.last_target.map_or_else(
            || vec![note(NOTE_NO_LAST_TARGET)],
            |last| vec![Effect::Act(Act::Attack(last))],
        ),
        ActionId::AttackSelected => {
            on_selected(context, |serial| vec![Effect::Act(Act::Attack(serial))])
        }
        ActionId::PrimaryAbility => vec![hotkey(HOTKEY_PRIMARY)],
        ActionId::SecondaryAbility => vec![hotkey(HOTKEY_SECONDARY)],
        ActionId::ArmDisarm => with_choice::<Hand>(words, |hand| {
            vec![hotkey(match hand {
                Hand::Left => HOTKEY_LEFT_HAND,
                Hand::Right => HOTKEY_RIGHT_HAND,
            })]
        }),
        ActionId::EquipLastWeapon => vec![Effect::Act(Act::WearLastWeapon)],
        ActionId::BandageSelf => vec![hotkey(HOTKEY_BANDAGE_SELF)],
        ActionId::BandageTarget => vec![hotkey(HOTKEY_BANDAGE_LAST)],
        ActionId::InvokeVirtue => {
            with_choice::<Virtue>(words, |virtue| vec![hotkey(Virtue::LABELS[virtue.index()])])
        }
        ActionId::TargetSystem => toggle(Switch::NewTargetSystem),
        ActionId::UseSkill => vec![command(format!("useskill {}", quoted(words)))],
        ActionId::LastSkill => vec![hotkey(HOTKEY_LAST_SKILL)],
        ActionId::CastSpell => vec![command(format!("cast {}", quoted(words)))],
        ActionId::LastSpell => vec![hotkey(HOTKEY_LAST_SPELL)],
        ActionId::LastTarget => vec![hotkey(HOTKEY_TARGET_LAST)],
        ActionId::TargetSelf => vec![hotkey(HOTKEY_TARGET_SELF)],
        ActionId::WaitForTarget => milliseconds(words, DEFAULT_TARGET_WAIT_MS).map_or_else(
            || {
                vec![note(format!(
                    "\"{words}\" is not a number of milliseconds."
                ))]
            },
            |most| vec![Effect::Wait(Wait::Target(most))],
        ),
        ActionId::TargetNext => vec![window(WindowCommand::Select(
            SelectHow::Next,
            SelectKind::Mobile,
        ))],
        ActionId::SelectNext => select(SelectHow::Next),
        ActionId::SelectPrevious => select(SelectHow::Previous),
        ActionId::SelectNearest => select(SelectHow::Nearest),
        ActionId::UseSelected => on_selected(context, |serial| vec![Effect::Act(Act::Use(serial))]),
        ActionId::TargetSelected => on_selected(context, |serial| {
            vec![
                Effect::Wait(Wait::Target(Duration::from_millis(DEFAULT_TARGET_WAIT_MS))),
                Effect::Act(Act::Target(serial)),
            ]
        }),
        ActionId::Grab => vec![window(WindowCommand::Aim(LocalAim::Grab))],
        ActionId::SetGrabBag => vec![window(WindowCommand::Aim(LocalAim::SetGrabBag))],
        ActionId::LastObject => vec![hotkey(HOTKEY_LAST_OBJECT)],
        ActionId::UseItemInHand => vec![command(COMMAND_USE_IN_HAND.to_string())],
        ActionId::UsePotion => vec![hotkey(&format!("{HOTKEY_POTION}{words}"))],
        ActionId::UseObject => with_choice(words, |object| use_object(object, frame)),
        ActionId::Delay => milliseconds(words, 0).map_or_else(
            || {
                vec![note(format!(
                    "\"{words}\" is not a number of milliseconds."
                ))]
            },
            |pause| vec![Effect::Wait(Wait::For(pause))],
        ),
        ActionId::RunScript => vec![command(play_line(words))],
        ActionId::Command => vec![command(words.to_string())],
        ActionId::Hotkey => vec![hotkey(words)],
        ActionId::LeftClick => vec![window(WindowCommand::Click(PointerClick::Left))],
        ActionId::RightClick => vec![window(WindowCommand::Click(PointerClick::Right))],
        ActionId::DoubleClick => vec![window(WindowCommand::Click(PointerClick::Double))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchContainer, WatchPackItem};
    use crate::window::actions::{ArgumentKind, ACTIONS};

    const BAG: u32 = 0x4000_0001;
    const ORC: u32 = 9;

    fn context<'a>(frame: &'a WatchFrame, profile: &'a Profile) -> Context<'a> {
        Context {
            frame,
            profile,
            selected: Some(ORC),
        }
    }

    /// An argument each kind of action takes.
    fn sample(kind: ArgumentKind) -> String {
        match kind {
            ArgumentKind::Number => "3".into(),
            ArgumentKind::Milliseconds => "250".into(),
            ArgumentKind::Text | ArgumentKind::Script | ArgumentKind::Command => "hail".into(),
            ArgumentKind::Hotkey => "Resync".into(),
            other => other.default_argument(),
        }
    }

    #[test]
    fn every_action_does_something_with_a_fitting_argument() {
        let frame = WatchFrame {
            last_target: Some(ORC),
            ..WatchFrame::default()
        };
        let profile = Profile::default();
        for spec in ACTIONS {
            let effects = resolve(
                spec.action,
                &sample(spec.argument),
                &context(&frame, &profile),
            );
            let noted = effects.iter().any(|e| matches!(e, Effect::Note(_)));
            assert!(!noted, "{}: {effects:?}", spec.id);
            let does_nothing = effects.is_empty();
            let held_or_shard_side = matches!(spec.action, ActionId::LookAtMouse);
            assert_eq!(does_nothing, held_or_shard_side, "{}", spec.id);
        }
    }

    #[test]
    fn a_wrong_argument_is_a_note_not_an_act() {
        let frame = WatchFrame::default();
        let profile = Profile::default();
        let context = context(&frame, &profile);
        for (action, argument) in [
            (ActionId::OpenGump, "no window"),
            (ActionId::Delay, "soon"),
            (ActionId::Walk, "up"),
            (ActionId::UseCounterSlot, "one"),
        ] {
            let effects = resolve(action, argument, &context);
            assert!(
                matches!(effects.as_slice(), [Effect::Note(_)]),
                "{action:?}"
            );
        }
    }

    #[test]
    fn the_click_to_run_key_switches_its_option() {
        let frame = WatchFrame::default();
        let profile = Profile::default();
        assert_eq!(
            resolve(ActionId::ClickToRun, "", &context(&frame, &profile)),
            vec![Effect::Window(WindowCommand::ToggleOption(
                Switch::ClickToRun
            ))]
        );
    }

    #[test]
    fn a_step_opens_doors_by_the_general_page_while_alive() {
        let mut frame = WatchFrame::default();
        let mut profile = Profile::default();
        assert!(!opens_doors(&frame, &profile));
        profile.general.auto_open_doors = true;
        assert!(opens_doors(&frame, &profile));
        frame.dead = true;
        assert!(!opens_doors(&frame, &profile), "a ghost opens no door");
    }

    #[test]
    fn a_walk_runs_only_when_always_run_allows_it() {
        let mut frame = WatchFrame {
            stam: 50,
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        assert!(!runs(&frame, &profile));
        profile.general.always_run = true;
        assert!(runs(&frame, &profile));
        frame.hidden = true;
        profile.general.always_run_unless_hidden = true;
        assert!(!runs(&frame, &profile));
        frame.hidden = false;
        frame.stam = 1;
        assert!(!runs(&frame, &profile));
        let context = context(&frame, &profile);
        assert_eq!(
            resolve(ActionId::Walk, "North-west", &context),
            vec![Effect::Act(Act::Step {
                direction: "nw",
                run: false,
                open_doors: false,
            })]
        );
    }

    #[test]
    fn a_backpack_opens_with_a_use_and_the_guild_with_a_request() {
        let mut frame = WatchFrame::default();
        frame.look.equipment.push(crate::view::WatchEquip {
            serial: BAG,
            layer: uoterm_protocol::types::LAYER_BACKPACK,
            ..Default::default()
        });
        let profile = Profile::default();
        let context = context(&frame, &profile);
        let opened = resolve(ActionId::OpenGump, "backpack", &context);
        assert_eq!(opened[0], Effect::Act(Act::Use(BAG)));
        assert_eq!(
            resolve(ActionId::OpenGump, "Guild", &context),
            vec![Effect::Act(Act::Command(COMMAND_GUILD.into()))]
        );
        assert!(resolve(ActionId::CloseGump, "Guild", &context).is_empty());
    }

    #[test]
    fn a_trapped_box_is_found_in_the_open_containers() {
        let frame = WatchFrame {
            containers: vec![WatchContainer {
                items: vec![WatchPackItem {
                    serial: BAG,
                    name: "a Trapped Box".into(),
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        let profile = Profile::default();
        assert_eq!(
            resolve(
                ActionId::UseObject,
                "Trapped box",
                &context(&frame, &profile)
            ),
            vec![Effect::Act(Act::Use(BAG))]
        );
    }
}
