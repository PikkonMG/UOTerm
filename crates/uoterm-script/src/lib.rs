//! The UOTerm script language.
//!
//! A script is one command on each line, with `if`, `while` and `for` blocks.
//! It follows the command style that UO assistant scripts have long used, so
//! scripts players already have will run. [`Program::parse`] reads a script.

mod error;
mod program;
mod run;
mod token;
mod vars;

pub use error::ParseError;
pub use program::{parse_number, Arg, Call, Condition, ForSpec, Join, Op, Operand, Program, Test};
pub use run::{Ctx, Host, Script, Status, Step, Value, MAX_STEPS_PER_TICK};
pub use token::{Compare, COMMENT};
pub use vars::Vars;
