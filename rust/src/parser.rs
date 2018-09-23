use ast::*;
use std::str::Chars;
use std::iter::Peekable;
use std::mem;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
    UnexpectedChar(char, Pos),
    UnexpectedEOF(Pos),
    UnexpectedEnd(Pos),
    MismatchingTags(String, String, Pos),
    MissingEnd(String, Pos),
    TagExpected(Pos),
    InvalidTagName(Pos),
}

struct State<'a> {
    filename: &'a str,
    line: u64,
    column: u64,
    chars: Peekable<Chars<'a>>,
}

impl<'a> State<'a> {
    fn next(&mut self) -> Option<char> {
        match self.chars.next() {
            None => None,
            Some(c) => {
                if c == '\n' {
                    self.column = 0;
                    self.line += 1;
                } else {
                    self.column += 1;
                }
                Some(c)
            }
        }
    }

    fn peek(&mut self) -> Option<char> {
        match self.chars.peek() {
            None => None,
            Some(&c) => Some(c)
        }
    }

    fn pos(&self) -> Pos {
        Pos {
            filename: self.filename.to_string(),
            line: self.line,
            column: self.column,
        }
    }

    fn eat<F>(&mut self, f: F) -> Result<char, Error>
        where F: Fn(char) -> bool {
        match self.next() {
            Some(c) if f(c) => Ok(c),
            Some(c) => Err(Error::UnexpectedChar(c, self.pos())),
            None => Err(Error::UnexpectedEOF(self.pos())),
        }
    }
}

pub fn parse_string(filename: &str, s: &str) -> Result<Doc, Error> {
    let mut state = State { filename, line: 0, column: 0, chars: s.chars().peekable() };
    let res = parse_doc(&mut state, None)?;
    match state.next() {
        None => Ok(res),
        Some(c) => Err(Error::UnexpectedChar(c, state.pos()))
    }
}

fn parse_doc(state: &mut State, required_end: Option<&str>) -> Result<Doc, Error> {

    let mut items = vec![];
    let mut text = String::new();
    let mut text_pos = state.pos();

    loop {

        let c = state.peek();

        match c {
            Some('\\') => {
                let pos = state.pos();

                if !text.is_empty() {
                    items.push(Item::Text(mem::replace(&mut text, String::new()), text_pos));
                };

                state.next();

                let mut tag = parse_tag(state)?;
                let mut have_begin = false;

                if tag == "begin" {
                    tag = parse_enclosed_tag(state)?;
                    have_begin = true;
                } else if tag == "end" {
                    tag = parse_enclosed_tag(state)?;
                    return match required_end {
                        Some(required_tag) if tag == required_tag => Ok(Doc(items)),
                        Some(required_tag) => Err(Error::MismatchingTags(required_tag.to_string(), tag, state.pos())),
                        None => Err(Error::UnexpectedEnd(state.pos())),
                    }
                }

                let mut named_args = HashMap::new();
                loop {
                    //skip_ws(state);
                    match state.peek() {
                        Some('[') => {
                            state.next();
                            skip_ws(state);
                            let tag = parse_regular_tag(state)?;
                            skip_ws(state);
                            state.eat(|c| c == '=')?;
                            named_args.insert(tag, parse_doc(state, None)?);
                            state.eat(|c| c == ']')?;
                        },
                        _ => break
                    }
                }

                let mut pos_args = vec![];
                loop {
                    //skip_ws(state);
                    match state.peek() {
                        Some('{') => {
                            state.next();
                            pos_args.push(parse_doc(state, None)?);
                            state.eat(|c| c == '}')?;
                        },
                        _ => break
                    }
                }

                if have_begin {
                    pos_args.push(parse_doc(state, Some(&tag))?);
                }

                items.push(Item::Element(Element { tag, named_args, pos_args, pos }));
                text_pos = state.pos().clone();
            }
            Some('{') => {
                state.next();
                state.eat(|c| c == '{')?;
                parse_raw(state, &mut text)?;
            }
            Some(c) if c != '{' && c != '}' && c != '[' && c != ']' => {
                state.next();
                text.push(c);
            }
            _ => {
                if required_end.is_some() {
                    return Err(Error::MissingEnd(required_end.unwrap().to_string(), state.pos()))
                }
                if !text.is_empty() {
                    items.push(Item::Text(text, text_pos));
                };
                return Ok(Doc(items));
            }
        }
    }
}

fn parse_raw(state: &mut State, res: &mut String) -> Result<(), Error> {
    loop {
        let c = state.eat(|_| true)?;
        match c {
            '{' => {
                let c2 = state.eat(|_| true)?;
                if c2 == '{' {
                    res.extend("{{".chars());
                    parse_raw(state, res)?;
                    res.extend("}}".chars());
                } else {
                    res.push(c);
                    res.push(c2);
                }
            }
            '}' => {
                let c2 = state.eat(|_| true)?;
                if c2 == '}' { return Ok(()) }
                res.push(c);
                res.push(c2);
            }
            _ => res.push(c)
        }
    }
}

fn parse_tag(state: &mut State) -> Result<Tag, Error> {
    let mut tag = Tag::new();
    loop {
        match state.peek() {
            Some(c) if c >= 'a' && c <= 'z' || c >= '0' && c <= '9' || c == '#' => {
                tag.push(c);
                state.next();
            },
            _ => {
                if tag.is_empty() {
                    return Err(Error::TagExpected(state.pos()))
                }
                return Ok(tag);
            }
        }
    }
}

fn parse_regular_tag(state: &mut State) -> Result<Tag, Error> {
    let tag = parse_tag(state)?;
    if tag == "begin" || tag == "end" {
        return Err(Error::InvalidTagName(state.pos()))
    }
    Ok(tag)
}

fn parse_enclosed_tag(state: &mut State) -> Result<Tag, Error> {
    //skip_ws(state);
    state.eat(|c| c == '{')?;
    //skip_ws(state);
    let tag = parse_regular_tag(state)?;
    //skip_ws(state);
    state.eat(|c| c == '}')?;
    Ok(tag)
}

fn skip_ws(state: &mut State) {
    loop {
        match state.peek() {
            Some(c) if c.is_whitespace() => state.next(),
            _ => return
        };
    }
}
