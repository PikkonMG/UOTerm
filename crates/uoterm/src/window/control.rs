//! The hand of the human on the character. The window turns a click or a
//! line of text into one act, and a worker thread makes the tool call, so
//! the window never waits for the session.
//!
//! Each call says `human: true`. The session lets those through while it
//! refuses the agent.

use super::orders;
use crate::remote;
use crate::view::WatchFrame;
use eframe::egui;
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use uoterm_runtime::tools::{
    ARG_HUMAN, TOOL_ATTACK, TOOL_DEPOSIT, TOOL_DROP, TOOL_FOLLOW, TOOL_GUMP_CLOSE,
    TOOL_GUMP_RESPOND, TOOL_LIFT, TOOL_LOOT, TOOL_MOVE_TO, TOOL_RELEASE_CONTROL, TOOL_SAY,
    TOOL_SINGLE_CLICK, TOOL_STOP, TOOL_TAKE_CONTROL, TOOL_TARGET, TOOL_USE, TOOL_WAR_MODE,
};

/// The shard refuses a drop that comes too soon after the lift.
const LIFT_TO_DROP: Duration = Duration::from_millis(650);
const ORDER_OFF: &str = "Orders need a TypeSafe key. Put TYPESAFE_API_KEY in the environment.";

/// One thing the human tells the character to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Take,
    GiveBack,
    WalkTo {
        x: u16,
        y: u16,
    },
    Use(u32),
    Look(u32),
    Attack(u32),
    Follow(u32),
    Loot(u32),
    Target(u32),
    TargetGround {
        x: u16,
        y: u16,
        z: i8,
    },
    CancelTarget,
    War(bool),
    Say(String),
    Stop,
    Deposit,
    /// Put an item of the pack on the ground at the feet of the character.
    PutDown(u32),
    GumpButton {
        gump: u32,
        button: u32,
        switches: Vec<u32>,
    },
    GumpClose(u32),
    /// Words for Jev to turn into one of the acts above.
    Order(String, Box<WatchFrame>),
}

/// What came of an act, in words for the human.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub text: String,
    pub failed: bool,
}

impl Act {
    /// The tool calls that make the act, in order.
    fn calls(&self) -> Vec<(&'static str, Value)> {
        let serial = |serial: &u32| json!({ "serial": serial });
        match self {
            Self::Take => vec![(TOOL_TAKE_CONTROL, json!({}))],
            Self::GiveBack => vec![(TOOL_RELEASE_CONTROL, json!({}))],
            Self::WalkTo { x, y } => vec![(TOOL_MOVE_TO, json!({ "x": x, "y": y }))],
            Self::Use(s) => vec![(TOOL_USE, serial(s))],
            Self::Look(s) => vec![(TOOL_SINGLE_CLICK, serial(s))],
            Self::Attack(s) => vec![(TOOL_ATTACK, serial(s))],
            Self::Follow(s) => vec![(TOOL_FOLLOW, serial(s))],
            Self::Loot(s) => vec![(TOOL_LOOT, serial(s))],
            Self::Target(s) => vec![(TOOL_TARGET, serial(s))],
            Self::TargetGround { x, y, z } => {
                vec![(TOOL_TARGET, json!({ "x": x, "y": y, "z": z }))]
            }
            Self::CancelTarget => vec![(TOOL_TARGET, json!({}))],
            Self::War(on) => vec![(TOOL_WAR_MODE, json!({ "on": on }))],
            Self::Say(text) => vec![(TOOL_SAY, json!({ "text": text }))],
            Self::Stop => vec![(TOOL_STOP, json!({}))],
            Self::Deposit => vec![(TOOL_DEPOSIT, json!({}))],
            Self::PutDown(s) => vec![(TOOL_LIFT, serial(s)), (TOOL_DROP, serial(s))],
            Self::GumpButton {
                gump,
                button,
                switches,
            } => vec![(
                TOOL_GUMP_RESPOND,
                json!({ "gump": gump, "button": button, "switches": switches }),
            )],
            Self::GumpClose(gump) => vec![(TOOL_GUMP_CLOSE, json!({ "gump": gump }))],
            Self::Order(..) => Vec::new(),
        }
    }

    /// The act in words, for the line that tells the human what was done.
    fn words(&self) -> String {
        match self {
            Self::Take => "You have the character.".into(),
            Self::GiveBack => "The agent has the character again.".into(),
            Self::WalkTo { x, y } => format!("Walk to {x}, {y}."),
            Self::Use(_) => "Use.".into(),
            Self::Look(_) => "Look.".into(),
            Self::Attack(_) => "Attack.".into(),
            Self::Follow(_) => "Follow.".into(),
            Self::Loot(_) => "Loot.".into(),
            Self::Target(_) | Self::TargetGround { .. } => "Target.".into(),
            Self::CancelTarget => "Target canceled.".into(),
            Self::War(true) => "War mode.".into(),
            Self::War(false) => "Peace mode.".into(),
            Self::Say(text) => format!("Said: {text}"),
            Self::Stop => "Stop.".into(),
            Self::Deposit => "Put the pack in the bank.".into(),
            Self::PutDown(_) => "Put the item down.".into(),
            Self::GumpButton { .. } => "Gump answered.".into(),
            Self::GumpClose(_) => "Gump closed.".into(),
            Self::Order(order, _) => format!("Order: {order}"),
        }
    }
}

/// The sending end the window holds, and the reports that come back.
pub struct Hand {
    acts: Sender<Act>,
    reports: Receiver<Report>,
    pub orders_on: bool,
}

impl Hand {
    pub fn start(api: String, session: String, ctx: egui::Context) -> Self {
        let (acts, inbox) = mpsc::channel();
        let (outbox, reports) = mpsc::channel();
        let key = orders::api_key();
        let orders_on = key.is_some();
        thread::spawn(move || work(&api, &session, key.as_deref(), &inbox, &outbox, &ctx));
        Self {
            acts,
            reports,
            orders_on,
        }
    }

    pub fn act(&self, act: Act) {
        // The worker lives as long as the window, so a send cannot fail.
        let _ = self.acts.send(act);
    }

    /// The newest report, when one came since the last frame.
    pub fn newest_report(&self) -> Option<Report> {
        self.reports.try_iter().last()
    }
}

fn work(
    api: &str,
    session: &str,
    key: Option<&str>,
    inbox: &Receiver<Act>,
    outbox: &Sender<Report>,
    ctx: &egui::Context,
) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    for act in inbox {
        let report = rt.block_on(perform(api, session, key, act));
        if outbox.send(report).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

async fn perform(api: &str, session: &str, key: Option<&str>, act: Act) -> Report {
    let failed = |text: String| Report { text, failed: true };
    let act = match act {
        Act::Order(order, frame) => {
            let Some(key) = key else {
                return failed(ORDER_OFF.into());
            };
            match orders::ask(key, &order, &frame).await {
                Ok(understood) => understood,
                Err(words) => return failed(words),
            }
        }
        plain => plain,
    };
    for (i, (tool, mut args)) in act.calls().into_iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(LIFT_TO_DROP).await;
        }
        args[ARG_HUMAN] = json!(true);
        if let Err(e) = remote::call_tool(api, session, tool, args).await {
            return failed(e.to_string());
        }
    }
    Report {
        text: act.words(),
        failed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: u32 = 0x4000_0001;

    #[test]
    fn an_act_is_the_tool_calls_the_session_knows() {
        assert_eq!(
            Act::WalkTo { x: 10, y: 20 }.calls(),
            vec![(TOOL_MOVE_TO, json!({ "x": 10, "y": 20 }))]
        );
        assert_eq!(Act::CancelTarget.calls(), vec![(TOOL_TARGET, json!({}))]);
        let put_down: Vec<&str> = Act::PutDown(ITEM).calls().iter().map(|c| c.0).collect();
        assert_eq!(put_down, vec![TOOL_LIFT, TOOL_DROP]);
    }

    #[test]
    fn each_tool_name_is_a_tool_of_the_session() {
        let listed = uoterm_runtime::tools::mcp_tool_list();
        let known: Vec<&str> = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        let acts = [
            Act::Take,
            Act::GiveBack,
            Act::WalkTo { x: 0, y: 0 },
            Act::Use(ITEM),
            Act::Look(ITEM),
            Act::Attack(ITEM),
            Act::Follow(ITEM),
            Act::Loot(ITEM),
            Act::Target(ITEM),
            Act::War(true),
            Act::Say("hail".into()),
            Act::Stop,
            Act::Deposit,
            Act::PutDown(ITEM),
            Act::GumpButton {
                gump: 1,
                button: 1,
                switches: Vec::new(),
            },
            Act::GumpClose(1),
        ];
        for act in acts {
            for (tool, _) in act.calls() {
                assert!(known.contains(&tool), "{tool} is not a session tool");
            }
        }
    }
}
