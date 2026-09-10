//! A script read into a flat list of steps. Blocks become jumps, so the
//! interpreter only ever moves a step pointer.

use crate::error::ParseError;
use crate::token::{tokenize, Compare, Token};

/// One argument as the script wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arg {
    pub text: String,
    /// It was written in quotes, so it is text even when it looks like a
    /// number or a keyword.
    pub quoted: bool,
}

const HEX_PREFIX: &str = "0x";
const HEX_RADIX: u32 = 16;

impl Arg {
    pub fn word(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            quoted: false,
        }
    }

    pub fn quoted(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            quoted: true,
        }
    }

    /// The argument as a number: decimal, negative, or `0x` hex. Quoted
    /// numbers count too, as scripts quote them often.
    pub fn number(&self) -> Option<i64> {
        parse_number(&self.text)
    }

    /// True when the argument is this word, in any case.
    pub fn is(&self, word: &str) -> bool {
        self.text.eq_ignore_ascii_case(word)
    }
}

/// A number as a script writes it: decimal, negative, or `0x` hex.
pub fn parse_number(text: &str) -> Option<i64> {
    let t = text.trim();
    let (negative, digits) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t),
    };
    let lower = digits.to_ascii_lowercase();
    let value = match lower.strip_prefix(HEX_PREFIX) {
        Some(hex) => i64::from_str_radix(hex, HEX_RADIX).ok()?,
        None => lower.parse::<i64>().ok()?,
    };
    Some(if negative { -value } else { value })
}

/// A command or a condition word with its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// The name in lower case, with the `@` and `!` marks taken off.
    pub name: String,
    pub args: Vec<Arg>,
    /// `@name`: say nothing when it finds nothing.
    pub quiet: bool,
    /// `name!`: the command's own stronger form.
    pub force: bool,
    pub line: usize,
}

/// The right side of a compare: a plain value, or a word the host reads, as
/// in `hits < maxhits`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Value(Arg),
    Call(Call),
}

/// One test of a condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Test {
    pub not: bool,
    pub left: Call,
    pub compare: Option<(Compare, Operand)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    And,
    Or,
}

/// Tests joined by `and` and `or`, read left to right with no grouping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
    pub first: Test,
    pub rest: Vec<(Join, Test)>,
}

/// The kinds of `for` loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForSpec {
    /// `for 5`: the body runs five times.
    Count(Arg),
    /// `for 1 to 10`.
    Range(Arg, Arg),
    /// `for 0 to 'list'`: each item of the list from that place on.
    List { start: Arg, list: Arg },
    /// `for 0 to 3 in 'list'`: the items from one place to another.
    ListRange { start: Arg, end: Arg, list: Arg },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Command(Call),
    /// Go to `jump` when the condition is false.
    If {
        cond: Condition,
        jump: usize,
        line: usize,
    },
    /// Go to `target`.
    Jump(usize),
    /// Go to `exit` when the condition is false. The body jumps back here.
    While {
        cond: Condition,
        exit: usize,
        line: usize,
    },
    /// Starts or goes on with loop number `slot`. When the loop is done, go to
    /// `exit`.
    For {
        spec: ForSpec,
        slot: usize,
        exit: usize,
        line: usize,
    },
    /// The end of a `for` body: count one pass and go back to `start`.
    ForNext {
        slot: usize,
        start: usize,
    },
    /// Forgets loop `slot`, so the next time the loop is reached it starts
    /// again from the top.
    ForDone {
        slot: usize,
    },
    Stop,
    Replay,
}

/// A script read and ready to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub ops: Vec<Op>,
    /// The number of `for` loops, one counter each.
    pub loops: usize,
}

const KW_IF: &str = "if";
const KW_ELSEIF: &str = "elseif";
const KW_ELSE: &str = "else";
const KW_ENDIF: &str = "endif";
const KW_WHILE: &str = "while";
const KW_ENDWHILE: &str = "endwhile";
const KW_FOR: &str = "for";
const KW_ENDFOR: &str = "endfor";
const KW_BREAK: &str = "break";
const KW_CONTINUE: &str = "continue";
const KW_STOP: &str = "stop";
const KW_REPLAY: &str = "replay";
const KW_NOT: &str = "not";
const KW_AND: &str = "and";
const KW_OR: &str = "or";
const KW_TO: &str = "to";
const KW_IN: &str = "in";
const QUIET_MARK: char = '@';
const FORCE_MARK: char = '!';
/// A jump not yet pointed anywhere. Every one is filled in before the
/// program is handed out.
const UNSET: usize = usize::MAX;

/// An open block while the script is read.
enum Block {
    If {
        /// The test that jumps when false, still to be pointed.
        pending: Option<usize>,
        /// The jumps out of each finished branch, all to the `endif`.
        ends: Vec<usize>,
        line: usize,
    },
    While {
        start: usize,
        breaks: Vec<usize>,
        line: usize,
    },
    For {
        start: usize,
        slot: usize,
        breaks: Vec<usize>,
        /// Jumps to the end of the body, where the pass is counted.
        continues: Vec<usize>,
        line: usize,
    },
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        let mut ops: Vec<Op> = Vec::new();
        let mut blocks: Vec<Block> = Vec::new();
        let mut loops = 0usize;
        let statements = source
            .lines()
            .enumerate()
            .flat_map(|(index, line)| statements(line).into_iter().map(move |s| (index + 1, s)));
        for (line_no, line) in statements {
            let tokens = tokenize(line, line_no)?;
            let Some(Token::Word(head)) = tokens.first() else {
                if tokens.is_empty() {
                    continue;
                }
                return Err(ParseError::new(line_no, "a line must start with a command"));
            };
            let head = head.to_ascii_lowercase();
            let rest = &tokens[1..];
            match head.as_str() {
                KW_IF => {
                    let cond = parse_condition(rest, line_no)?;
                    blocks.push(Block::If {
                        pending: Some(ops.len()),
                        ends: Vec::new(),
                        line: line_no,
                    });
                    ops.push(Op::If {
                        cond,
                        jump: UNSET,
                        line: line_no,
                    });
                }
                KW_ELSEIF => elseif(&mut ops, &mut blocks, rest, line_no)?,
                KW_ELSE if rest.first().is_some_and(|t| is_word(t, KW_IF)) => {
                    elseif(&mut ops, &mut blocks, &rest[1..], line_no)?
                }
                KW_ELSE => {
                    let Some(Block::If { pending, ends, .. }) = blocks.last_mut() else {
                        return Err(ParseError::new(line_no, "else with no if"));
                    };
                    let Some(test) = pending.take() else {
                        return Err(ParseError::new(line_no, "a second else in one if"));
                    };
                    ends.push(ops.len());
                    ops.push(Op::Jump(UNSET));
                    let here = ops.len();
                    point(&mut ops, test, here);
                }
                KW_ENDIF => {
                    let Some(Block::If { pending, ends, .. }) = blocks.pop() else {
                        return Err(ParseError::new(line_no, "endif with no if"));
                    };
                    let here = ops.len();
                    if let Some(test) = pending {
                        point(&mut ops, test, here);
                    }
                    for jump in ends {
                        point(&mut ops, jump, here);
                    }
                }
                KW_WHILE => {
                    let cond = parse_condition(rest, line_no)?;
                    blocks.push(Block::While {
                        start: ops.len(),
                        breaks: Vec::new(),
                        line: line_no,
                    });
                    ops.push(Op::While {
                        cond,
                        exit: UNSET,
                        line: line_no,
                    });
                }
                KW_ENDWHILE => {
                    let Some(Block::While { start, breaks, .. }) = blocks.pop() else {
                        return Err(ParseError::new(line_no, "endwhile with no while"));
                    };
                    ops.push(Op::Jump(start));
                    let exit = ops.len();
                    point(&mut ops, start, exit);
                    for jump in breaks {
                        point(&mut ops, jump, exit);
                    }
                }
                KW_FOR => {
                    let spec = parse_for(rest, line_no)?;
                    let slot = loops;
                    loops += 1;
                    blocks.push(Block::For {
                        start: ops.len(),
                        slot,
                        breaks: Vec::new(),
                        continues: Vec::new(),
                        line: line_no,
                    });
                    ops.push(Op::For {
                        spec,
                        slot,
                        exit: UNSET,
                        line: line_no,
                    });
                }
                KW_ENDFOR => {
                    let Some(Block::For {
                        start,
                        slot,
                        breaks,
                        continues,
                        ..
                    }) = blocks.pop()
                    else {
                        return Err(ParseError::new(line_no, "endfor with no for"));
                    };
                    let next = ops.len();
                    for jump in continues {
                        point(&mut ops, jump, next);
                    }
                    ops.push(Op::ForNext { slot, start });
                    let done = ops.len();
                    ops.push(Op::ForDone { slot });
                    point(&mut ops, start, done);
                    for jump in breaks {
                        point(&mut ops, jump, done);
                    }
                }
                KW_BREAK => {
                    no_arguments(rest, line_no, KW_BREAK)?;
                    let here = ops.len();
                    match innermost_loop(&mut blocks) {
                        Some(Block::While { breaks, .. } | Block::For { breaks, .. }) => {
                            breaks.push(here)
                        }
                        _ => return Err(ParseError::new(line_no, "break outside a loop")),
                    }
                    ops.push(Op::Jump(UNSET));
                }
                KW_CONTINUE => {
                    no_arguments(rest, line_no, KW_CONTINUE)?;
                    let here = ops.len();
                    let target = match innermost_loop(&mut blocks) {
                        Some(Block::While { start, .. }) => *start,
                        // The end of the body counts the pass, then goes back.
                        Some(Block::For { continues, .. }) => {
                            continues.push(here);
                            UNSET
                        }
                        _ => return Err(ParseError::new(line_no, "continue outside a loop")),
                    };
                    ops.push(Op::Jump(target));
                }
                KW_STOP => {
                    no_arguments(rest, line_no, KW_STOP)?;
                    ops.push(Op::Stop);
                }
                KW_REPLAY => {
                    no_arguments(rest, line_no, KW_REPLAY)?;
                    ops.push(Op::Replay);
                }
                _ => ops.push(Op::Command(parse_call(&tokens, line_no)?)),
            }
        }
        if let Some(open) = blocks.last() {
            let (what, line) = match open {
                Block::If { line, .. } => ("if with no endif", *line),
                Block::While { line, .. } => ("while with no endwhile", *line),
                Block::For { line, .. } => ("for with no endfor", *line),
            };
            return Err(ParseError::new(line, what));
        }
        Ok(Self { ops, loops })
    }
}

/// The statements of one line: `;` puts more than one on a line. A `;` in
/// quotes is text, and a note ends the line.
fn statements(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if line[i..].starts_with(crate::token::COMMENT) => {
                out.push(&line[start..i]);
                return out;
            }
            None if c == STATEMENT_BREAK => {
                out.push(&line[start..i]);
                start = i + 1;
            }
            None => {}
        }
    }
    out.push(&line[start..]);
    out
}

/// Puts a second statement on the same line.
const STATEMENT_BREAK: char = ';';

fn elseif(
    ops: &mut Vec<Op>,
    blocks: &mut [Block],
    rest: &[Token],
    line_no: usize,
) -> Result<(), ParseError> {
    let cond = parse_condition(rest, line_no)?;
    let Some(Block::If { pending, ends, .. }) = blocks.last_mut() else {
        return Err(ParseError::new(line_no, "elseif with no if"));
    };
    let Some(test) = pending.take() else {
        return Err(ParseError::new(line_no, "elseif after else"));
    };
    ends.push(ops.len());
    ops.push(Op::Jump(UNSET));
    let here = ops.len();
    point(ops, test, here);
    *pending = Some(here);
    ops.push(Op::If {
        cond,
        jump: UNSET,
        line: line_no,
    });
    Ok(())
}

fn innermost_loop(blocks: &mut [Block]) -> Option<&mut Block> {
    blocks
        .iter_mut()
        .rev()
        .find(|b| matches!(b, Block::While { .. } | Block::For { .. }))
}

/// Points the jump at `op` to `target`.
fn point(ops: &mut [Op], op: usize, target: usize) {
    match &mut ops[op] {
        Op::If { jump, .. } | Op::Jump(jump) => *jump = target,
        Op::While { exit, .. } | Op::For { exit, .. } => *exit = target,
        other => unreachable!("only jumps are pointed, not {other:?}"),
    }
}

fn is_word(token: &Token, word: &str) -> bool {
    matches!(token, Token::Word(w) if w.eq_ignore_ascii_case(word))
}

fn no_arguments(rest: &[Token], line_no: usize, keyword: &str) -> Result<(), ParseError> {
    if rest.is_empty() {
        Ok(())
    } else {
        Err(ParseError::new(
            line_no,
            format!("{keyword} takes nothing after it"),
        ))
    }
}

fn to_arg(token: &Token, line_no: usize) -> Result<Arg, ParseError> {
    match token {
        Token::Word(w) => Ok(Arg::word(w.clone())),
        Token::Quoted(q) => Ok(Arg::quoted(q.clone())),
        Token::Compare(c) => Err(ParseError::new(
            line_no,
            format!("{} is only for conditions", c.sign()),
        )),
    }
}

/// A command and its arguments, with the `@` and `!` marks read off the name.
fn parse_call(tokens: &[Token], line_no: usize) -> Result<Call, ParseError> {
    let Some(Token::Word(raw)) = tokens.first() else {
        return Err(ParseError::new(line_no, "a command name is missing"));
    };
    let quiet = raw.starts_with(QUIET_MARK);
    let name = raw.trim_start_matches(QUIET_MARK);
    let force = name.ends_with(FORCE_MARK);
    let name = name.trim_end_matches(FORCE_MARK).to_ascii_lowercase();
    if name.is_empty() {
        return Err(ParseError::new(line_no, "a command name is missing"));
    }
    let args = tokens[1..]
        .iter()
        .map(|t| to_arg(t, line_no))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Call {
        name,
        args,
        quiet,
        force,
        line: line_no,
    })
}

fn parse_condition(tokens: &[Token], line_no: usize) -> Result<Condition, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::new(line_no, "a condition is missing"));
    }
    let mut parts: Vec<(Option<Join>, &[Token])> = Vec::new();
    let mut start = 0;
    let mut join = None;
    for (i, token) in tokens.iter().enumerate() {
        let next = if is_word(token, KW_AND) {
            Some(Join::And)
        } else if is_word(token, KW_OR) {
            Some(Join::Or)
        } else {
            None
        };
        if let Some(next) = next {
            parts.push((join, &tokens[start..i]));
            join = Some(next);
            start = i + 1;
        }
    }
    parts.push((join, &tokens[start..]));
    let mut tests = parts
        .into_iter()
        .map(|(join, part)| parse_test(part, line_no).map(|test| (join, test)));
    let (_, first) = tests.next().expect("a condition always has a first part")?;
    let rest = tests
        .map(|r| r.map(|(join, test)| (join.expect("every later part follows a join"), test)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Condition { first, rest })
}

fn parse_test(tokens: &[Token], line_no: usize) -> Result<Test, ParseError> {
    let (not, tokens) = match tokens.first() {
        Some(t) if is_word(t, KW_NOT) => (true, &tokens[1..]),
        _ => (false, tokens),
    };
    if tokens.is_empty() {
        return Err(ParseError::new(line_no, "a test is missing its word"));
    }
    let split = tokens.iter().position(|t| matches!(t, Token::Compare(_)));
    let Some(at) = split else {
        return Ok(Test {
            not,
            left: parse_call(tokens, line_no)?,
            compare: None,
        });
    };
    let Token::Compare(compare) = tokens[at] else {
        unreachable!("the split is on a compare sign");
    };
    let left = parse_call(&tokens[..at], line_no)?;
    let right = &tokens[at + 1..];
    let operand = match right {
        [] => {
            return Err(ParseError::new(
                line_no,
                "a compare has nothing on its right",
            ))
        }
        [Token::Quoted(q)] => Operand::Value(Arg::quoted(q.clone())),
        [Token::Word(w)] if parse_number(w).is_some() => Operand::Value(Arg::word(w.clone())),
        _ => Operand::Call(parse_call(right, line_no)?),
    };
    Ok(Test {
        not,
        left,
        compare: Some((compare, operand)),
    })
}

fn parse_for(tokens: &[Token], line_no: usize) -> Result<ForSpec, ParseError> {
    let args: Vec<Arg> = tokens
        .iter()
        .map(|t| to_arg(t, line_no))
        .collect::<Result<_, _>>()?;
    let word = |i: usize, w: &str| args.get(i).is_some_and(|a| !a.quoted && a.is(w));
    match args.len() {
        1 => Ok(ForSpec::Count(args[0].clone())),
        3 if word(1, KW_TO) && args[2].quoted => Ok(ForSpec::List {
            start: args[0].clone(),
            list: args[2].clone(),
        }),
        3 if word(1, KW_TO) => Ok(ForSpec::Range(args[0].clone(), args[2].clone())),
        5 if word(1, KW_TO) && word(3, KW_IN) => Ok(ForSpec::ListRange {
            start: args[0].clone(),
            end: args[2].clone(),
            list: args[4].clone(),
        }),
        _ => Err(ParseError::new(
            line_no,
            "for takes a count, or 'start to end', or 'start to list', or 'start to end in list'",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(source: &str) -> Program {
        Program::parse(source).expect("a script")
    }

    fn cmd(op: &Op) -> &str {
        match op {
            Op::Command(c) => &c.name,
            other => panic!("not a command: {other:?}"),
        }
    }

    #[test]
    fn a_command_keeps_its_marks_and_arguments() {
        let p = program("@usetype! 0xE21 'any' 'ground' 2");
        let Op::Command(call) = &p.ops[0] else {
            panic!("a command");
        };
        assert_eq!(call.name, "usetype");
        assert!(call.quiet && call.force);
        assert_eq!(call.args.len(), 4);
        assert_eq!(call.args[0].number(), Some(0xE21));
        assert!(call.args[1].quoted);
    }

    #[test]
    fn numbers_read_in_decimal_hex_and_below_zero() {
        assert_eq!(parse_number("0x40116650"), Some(0x4011_6650));
        assert_eq!(parse_number("-1"), Some(-1));
        assert_eq!(parse_number("12"), Some(12));
        assert_eq!(parse_number("any"), None);
    }

    #[test]
    fn an_if_with_elseif_and_else_jumps_past_the_other_branches() {
        let p = program(
            "if poisoned\n  cast 'Cure'\nelseif hits < maxhits\n  bandageself\nelse\n  msg 'fine'\nendif\npause 500",
        );
        // 0 If, 1 cast, 2 Jump, 3 If(elseif), 4 bandage, 5 Jump, 6 msg, 7 pause
        assert!(matches!(p.ops[0], Op::If { jump: 3, .. }));
        assert_eq!(cmd(&p.ops[1]), "cast");
        assert_eq!(p.ops[2], Op::Jump(7));
        assert!(matches!(p.ops[3], Op::If { jump: 6, .. }));
        assert_eq!(p.ops[5], Op::Jump(7));
        assert_eq!(cmd(&p.ops[6]), "msg");
        assert_eq!(cmd(&p.ops[7]), "pause");
    }

    #[test]
    fn else_if_in_two_words_is_an_elseif() {
        assert_eq!(
            program("if dead\nelse if hidden\nendif").ops,
            program("if dead\nelseif hidden\nendif").ops
        );
    }

    #[test]
    fn a_while_loops_back_and_break_leaves_it() {
        let p = program("while not dead\n  if hidden\n    break\n  endif\n  pause 100\nendwhile");
        // 0 While, 1 If, 2 Jump(break), 3 pause, 4 Jump(0)
        assert!(matches!(p.ops[0], Op::While { exit: 5, .. }));
        assert!(matches!(p.ops[1], Op::If { jump: 3, .. }));
        assert_eq!(p.ops[2], Op::Jump(5));
        assert_eq!(p.ops[4], Op::Jump(0));
    }

    #[test]
    fn a_for_loop_counts_and_forgets_its_counter_at_the_end() {
        let p = program("for 3\n  msg 'hi'\nendfor");
        assert!(matches!(
            p.ops[0],
            Op::For {
                spec: ForSpec::Count(_),
                slot: 0,
                exit: 3,
                ..
            }
        ));
        assert_eq!(p.ops[2], Op::ForNext { slot: 0, start: 0 });
        assert_eq!(p.ops[3], Op::ForDone { slot: 0 });
        assert_eq!(p.loops, 1);
    }

    #[test]
    fn continue_in_a_for_loop_goes_to_the_pass_count() {
        let p = program(
            "for 3
  continue
  msg 'never'
endfor",
        );
        // 0 For, 1 Jump(continue), 2 msg, 3 ForNext, 4 ForDone
        assert_eq!(p.ops[1], Op::Jump(3));
        assert_eq!(p.ops[3], Op::ForNext { slot: 0, start: 0 });
    }

    #[test]
    fn every_for_form_is_read() {
        let spec = |s: &str| match &program(&format!("{s}\nendfor")).ops[0] {
            Op::For { spec, .. } => spec.clone(),
            other => panic!("{other:?}"),
        };
        assert!(matches!(spec("for 1 to 10"), ForSpec::Range(..)));
        assert!(matches!(spec("for 0 to 'list'"), ForSpec::List { .. }));
        assert!(matches!(
            spec("for 0 to 2 in 'list'"),
            ForSpec::ListRange { .. }
        ));
    }

    #[test]
    fn a_condition_splits_on_and_or_and_reads_both_sides_of_a_compare() {
        let p = program("if not poisoned 'self' and hits < maxhits or counttype 0xf0c 'any' 'backpack' > 10\nendif");
        let Op::If { cond, .. } = &p.ops[0] else {
            panic!("an if");
        };
        assert!(cond.first.not);
        assert_eq!(cond.first.left.name, "poisoned");
        assert_eq!(cond.rest.len(), 2);
        let (join, hits) = &cond.rest[0];
        assert_eq!(*join, Join::And);
        assert!(matches!(
            &hits.compare,
            Some((Compare::Less, Operand::Call(c))) if c.name == "maxhits"
        ));
        let (join, count) = &cond.rest[1];
        assert_eq!(*join, Join::Or);
        assert_eq!(count.left.args.len(), 3);
        assert!(
            matches!(&count.compare, Some((Compare::Greater, Operand::Value(v))) if v.number() == Some(10))
        );
    }

    #[test]
    fn a_semicolon_puts_two_statements_on_one_line() {
        let p = program("msg 'a;b'; pause 100 // note; not a statement");
        assert_eq!(p.ops.len(), 2);
        let Op::Command(first) = &p.ops[0] else {
            panic!("a command");
        };
        assert_eq!(first.args[0].text, "a;b");
        assert_eq!(cmd(&p.ops[1]), "pause");
    }

    #[test]
    fn broken_blocks_name_their_line() {
        assert_eq!(Program::parse("if dead").unwrap_err().line, 1);
        assert_eq!(Program::parse("msg 'a'\nendif").unwrap_err().line, 2);
        assert_eq!(Program::parse("while dead\nendfor").unwrap_err().line, 2);
        assert_eq!(Program::parse("break").unwrap_err().line, 1);
        assert_eq!(
            Program::parse("if dead\nelse\nelse\nendif")
                .unwrap_err()
                .line,
            3
        );
    }
}
