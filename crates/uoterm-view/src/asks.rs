//! The asks of the panels, apart from how a window sends them: the calls
//! each ask makes, one after the other, and the answer they come to. The
//! Rust window makes the calls on its worker thread; the web view gives
//! them to the page. Both read the results here, so an answer has one
//! shape.

use crate::act::{places_in, status_words, string_list, Answer, Ask};
use crate::orders::PickQuestion;
use serde_json::{json, Value};
use uoterm_world::tool_names::{
    TOOL_FIND_LANDMARKS, TOOL_LIST_SCRIPTS, TOOL_SCRIPT_READ, TOOL_SCRIPT_STATUS,
};

/// What the human reads when Jev picks none of the names.
pub const NOT_SURE: &str = "Jev is not sure which one you mean. Pick it from the list.";
/// What the human reads when the markers name no place on the map.
pub const NO_PLACE_ON_MAP: &str = "The marker file names no place on this map.";
/// What the human reads when Jev picks none of the places.
pub const NO_SUCH_PLACE: &str = "Jev is not sure which place you mean. Click the map instead.";
const KEY_SCRIPTS: &str = "scripts";
const KEY_TEXT: &str = "text";

/// One call an ask makes.
#[derive(Clone, Debug, PartialEq)]
pub enum AskCall {
    /// A tool of the session that reads.
    Read { tool: &'static str, args: Value },
    /// Jev picks which of `names` the wish names.
    Pick {
        question: PickQuestion,
        wish: String,
        names: Vec<String>,
    },
    /// Jev turns the wish into the script lines of one hotkey.
    Lines { wish: String },
}

/// What a call came to.
#[derive(Clone, Debug, PartialEq)]
pub enum CallResult {
    /// The answer of a tool, or the words of its fault.
    Read(Result<Value, String>),
    /// The place of the name Jev picked, None when it is not sure, or the
    /// words of the fault.
    Picked(Result<Option<usize>, String>),
    /// The script lines, or the words of the fault.
    Lines(Result<String, String>),
}

/// What comes after a call: the next call, or the answer of the ask.
#[derive(Clone, Debug, PartialEq)]
pub enum AskStep {
    Call(AskCall),
    Done(Answer),
}

/// One ask on its way: what it asked, and the places it read on the way.
#[derive(Clone, Debug, PartialEq)]
pub struct AskRun {
    ask: Ask,
    /// The named places a place on the map is picked from.
    places: Vec<(String, u16, u16)>,
}

/// The call that asks Jev to pick one of `names`.
fn pick(question: PickQuestion, wish: &str, names: &[String]) -> AskCall {
    AskCall::Pick {
        question,
        wish: wish.to_string(),
        names: names.to_vec(),
    }
}

impl AskRun {
    /// Starts an ask: the run, and its first call.
    pub fn start(ask: Ask) -> (Self, AskCall) {
        let read = |tool, args| AskCall::Read { tool, args };
        let call = match &ask {
            Ask::Scripts => read(TOOL_LIST_SCRIPTS, json!({})),
            Ask::ScriptText(name) => read(TOOL_SCRIPT_READ, json!({ "name": name })),
            Ask::ScriptStatus => read(TOOL_SCRIPT_STATUS, json!({})),
            Ask::LinesFor(wish) => AskCall::Lines { wish: wish.clone() },
            Ask::PlaceOnMap { map, .. } => read(TOOL_FIND_LANDMARKS, json!({ "map": map })),
            Ask::HousePart { wish, options } => pick(PickQuestion::HousePart, wish, options),
            Ask::WearItem { wish, options } => pick(PickQuestion::Wear, wish, options),
            Ask::Channel { wish, options } => pick(PickQuestion::Channel, wish, options),
        };
        let run = Self {
            ask,
            places: Vec::new(),
        };
        (run, call)
    }

    /// Takes the result of the last call: gives the next call, or the
    /// answer.
    pub fn next(&mut self, result: CallResult) -> AskStep {
        let answer = match (&self.ask, result) {
            (Ask::Scripts, CallResult::Read(read)) => {
                Answer::Scripts(string_list(read.ok().as_ref(), KEY_SCRIPTS))
            }
            (Ask::ScriptText(name), CallResult::Read(read)) => Answer::ScriptText {
                name: name.clone(),
                text: read
                    .ok()
                    .and_then(|value| {
                        value
                            .get(KEY_TEXT)
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_default(),
            },
            (Ask::ScriptStatus, CallResult::Read(read)) => {
                Answer::ScriptStatus(status_words(read.ok().as_ref()))
            }
            (Ask::LinesFor(_), CallResult::Lines(lines)) => Answer::Lines(lines),
            (
                Ask::PlaceOnMap {
                    wish,
                    map,
                    from,
                    to,
                },
                CallResult::Read(read),
            ) => {
                let landmarks = match read {
                    Ok(landmarks) => landmarks,
                    Err(words) => return AskStep::Done(Answer::Place(Err(words))),
                };
                self.places = places_in(&landmarks, *map, *from, *to);
                if self.places.is_empty() {
                    return AskStep::Done(Answer::Place(Err(NO_PLACE_ON_MAP.into())));
                }
                return AskStep::Call(AskCall::Pick {
                    question: PickQuestion::Landmark,
                    wish: wish.clone(),
                    names: self.places.iter().map(|(name, ..)| name.clone()).collect(),
                });
            }
            (Ask::PlaceOnMap { .. }, CallResult::Picked(picked)) => Answer::Place(
                picked
                    .and_then(|place| place.ok_or_else(|| NO_SUCH_PLACE.to_string()))
                    .and_then(|place| {
                        self.places
                            .get(place)
                            .map(|(_, x, y)| (*x, *y))
                            .ok_or_else(|| NO_SUCH_PLACE.to_string())
                    }),
            ),
            (_, CallResult::Picked(picked)) => {
                Answer::Picked(picked.and_then(|place| place.ok_or_else(|| NOT_SURE.to_string())))
            }
            // A result of another kind of call than the ask made answers
            // nothing it asked: the ask ends with no answer of its own.
            (ask, _) => fault_answer(ask, String::new()),
        };
        AskStep::Done(answer)
    }
}

/// The answer of an ask that came to nothing, with the words of the fault.
fn fault_answer(ask: &Ask, words: String) -> Answer {
    match ask {
        Ask::Scripts => Answer::Scripts(Vec::new()),
        Ask::ScriptText(name) => Answer::ScriptText {
            name: name.clone(),
            text: String::new(),
        },
        Ask::ScriptStatus => Answer::ScriptStatus(String::new()),
        Ask::LinesFor(_) => Answer::Lines(Err(words)),
        Ask::PlaceOnMap { .. } => Answer::Place(Err(words)),
        _ => Answer::Picked(Err(words)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: u8 = 1;

    fn done(run: &mut AskRun, result: CallResult) -> Answer {
        match run.next(result) {
            AskStep::Done(answer) => answer,
            AskStep::Call(call) => panic!("one more call: {call:?}"),
        }
    }

    #[test]
    fn a_script_ask_reads_one_tool_and_takes_its_list() {
        let (mut run, call) = AskRun::start(Ask::Scripts);
        assert_eq!(
            call,
            AskCall::Read {
                tool: TOOL_LIST_SCRIPTS,
                args: json!({})
            }
        );
        let answer = done(
            &mut run,
            CallResult::Read(Ok(json!({"scripts": ["a", "b"]}))),
        );
        assert_eq!(answer, Answer::Scripts(vec!["a".into(), "b".into()]));
        let (mut run, _) = AskRun::start(Ask::ScriptText("a".into()));
        let failed = done(&mut run, CallResult::Read(Err("gone".into())));
        assert_eq!(
            failed,
            Answer::ScriptText {
                name: "a".into(),
                text: String::new()
            }
        );
    }

    #[test]
    fn a_pick_jev_is_not_sure_of_tells_the_human() {
        let ask = Ask::WearItem {
            wish: "my sword".into(),
            options: vec!["sword".into(), "shield".into()],
        };
        let (mut run, call) = AskRun::start(ask.clone());
        assert_eq!(
            call,
            AskCall::Pick {
                question: PickQuestion::Wear,
                wish: "my sword".into(),
                names: vec!["sword".into(), "shield".into()],
            }
        );
        assert_eq!(
            done(&mut run, CallResult::Picked(Ok(Some(0)))),
            Answer::Picked(Ok(0))
        );
        let (mut run, _) = AskRun::start(ask);
        assert_eq!(
            done(&mut run, CallResult::Picked(Ok(None))),
            Answer::Picked(Err(NOT_SURE.into()))
        );
    }

    #[test]
    fn a_place_on_the_map_reads_the_landmarks_then_picks_one() {
        let (mut run, call) = AskRun::start(Ask::PlaceOnMap {
            wish: "the bank".into(),
            map: MAP,
            from: (0, 0),
            to: (100, 100),
        });
        assert!(matches!(call, AskCall::Read { tool, .. } if tool == TOOL_FIND_LANDMARKS));
        let landmarks = json!([
            {"name": "Bank", "map": MAP, "location": {"x": 10, "y": 20}},
            {"name": "Far", "map": MAP, "location": {"x": 900, "y": 900}},
        ]);
        let AskStep::Call(AskCall::Pick {
            names, question, ..
        }) = run.next(CallResult::Read(Ok(landmarks)))
        else {
            panic!("Jev picks next");
        };
        assert_eq!(
            (question, names),
            (PickQuestion::Landmark, vec!["Bank".to_string()])
        );
        assert_eq!(
            done(&mut run, CallResult::Picked(Ok(Some(0)))),
            Answer::Place(Ok((10, 20)))
        );
    }

    #[test]
    fn a_place_with_no_landmark_on_the_map_is_told_at_once() {
        let (mut run, _) = AskRun::start(Ask::PlaceOnMap {
            wish: "x".into(),
            map: MAP,
            from: (0, 0),
            to: (1, 1),
        });
        assert_eq!(
            done(&mut run, CallResult::Read(Ok(json!([])))),
            Answer::Place(Err(NO_PLACE_ON_MAP.into()))
        );
    }

    #[test]
    fn lines_come_as_they_are() {
        let (mut run, call) = AskRun::start(Ask::LinesFor("heal".into()));
        assert_eq!(
            call,
            AskCall::Lines {
                wish: "heal".into()
            }
        );
        assert_eq!(
            done(&mut run, CallResult::Lines(Err("off".into()))),
            Answer::Lines(Err("off".into()))
        );
    }
}
