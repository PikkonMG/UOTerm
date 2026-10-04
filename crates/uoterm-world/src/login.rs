//! What a login asks a person, and what the person answers. The window and
//! the browser page speak these across the wire as JSON, so they live below
//! the runtime that owns the login itself.

use serde::{Deserialize, Serialize};
use uoterm_protocol::StartTown;

/// What a login screen may ask the shard to do with the characters of the
/// account, before it plays one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterRequest {
    /// Play the character in this slot.
    Play(usize),
    Delete(usize),
    Make(Box<NewCharacterWish>),
    /// Play none: the login ends at the character list.
    Leave,
}

/// What the shard lets a new character be: the towns he may start in, the
/// features of the account (`0xB9`) and the flags of the character list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterChoices {
    pub towns: Vec<StartTown>,
    pub features: u32,
    pub list_flags: u32,
}

/// What a player picked for a new character, in the words of a screen.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCharacterWish {
    pub name: String,
    pub female: bool,
    pub race: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub skills: Vec<(u8, u8)>,
    pub skin_hue: u16,
    pub hair: u16,
    pub hair_hue: u16,
    pub beard: u16,
    pub beard_hue: u16,
    pub shirt_hue: u16,
    pub pants_hue: u16,
    pub profession: u8,
    pub start_city: u16,
    pub slot: u16,
}

/// What a login asks a human: which shard of the list, and what to do with
/// the characters of the account. The answer is a [`LoginReply`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LoginAsk {
    Shard {
        names: Vec<String>,
    },
    /// The characters of the account. The screen may play one, delete one
    /// or make a new one. The shard answers a new list, or a refusal.
    Characters {
        names: Vec<String>,
        /// The words of the last refusal of the shard, when there was one.
        refused: Option<String>,
        /// What a new character may be: the start towns and the flags.
        choices: CharacterChoices,
    },
}

/// The answer to a [`LoginAsk`]: the place of the shard in the names, or
/// what to do with the characters of the account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LoginReply {
    Pick { index: usize },
    Request { request: CharacterRequest },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_shard_question_goes_on_the_wire_by_kind() {
        let ask = LoginAsk::Shard {
            names: vec!["Atlantic".into()],
        };
        assert_eq!(
            serde_json::to_value(&ask).unwrap(),
            json!({"kind": "Shard", "names": ["Atlantic"]})
        );
    }

    #[test]
    fn a_make_request_comes_back_from_the_wire() {
        let wish = NewCharacterWish {
            name: "Mara".into(),
            ..NewCharacterWish::default()
        };
        let reply = LoginReply::Request {
            request: CharacterRequest::Make(Box::new(wish.clone())),
        };
        let text = serde_json::to_string(&reply).unwrap();
        let back: LoginReply = serde_json::from_str(&text).unwrap();
        assert_eq!(
            back,
            LoginReply::Request {
                request: CharacterRequest::Make(Box::new(wish))
            }
        );
    }
}
