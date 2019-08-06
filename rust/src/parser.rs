use crate::ast::*;
use std::collections::HashMap;
use std::iter::Peekable;
use std::mem;
use std::str::Chars;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
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
    filename: Option<Arc<String>>,
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
            Some(&c) => Some(c),
        }
    }

    fn pos(&self) -> Pos {
        Pos {
            filename: self.filename.clone(),
            line: self.line,
            column: self.column,
        }
    }

    fn eat<F>(&mut self, f: F) -> Result<char, Error>
    where
        F: Fn(char) -> bool,
    {
        match self.next() {
            Some(c) if f(c) => Ok(c),
            Some(c) => Err(Error::UnexpectedChar(c, self.pos())),
            None => Err(Error::UnexpectedEOF(self.pos())),
        }
    }
}

pub fn parse_string(filename: Option<&str>, s: &str) -> Result<Doc, Error> {
    let mut state = State {
        filename: filename.map(|filename| Arc::new(filename.to_string())),
        line: 0,
        column: 0,
        chars: s.chars().peekable(),
    };
    let (res, _) = parse_doc(&mut state, None)?;
    match state.next() {
        None => Ok(res),
        Some(c) => Err(Error::UnexpectedChar(c, state.pos())),
    }
}

#[derive(Debug, Clone)]
struct Indent {
    open: bool,
    s: String,
}

impl Indent {
    fn new() -> Self {
        Indent {
            open: true,
            s: "".to_string(),
        }
    }
}

fn parse_doc<'a>(state: &mut State, required_end: Option<&str>) -> Result<(Doc, Indent), Error> {
    let (mut doc, indent) = parse_doc2(state, required_end)?;

    /* Strip leading empty line. */
    if !doc.is_empty() {
        if let Item::Text(ref mut s, _) = doc[0] {
            let mut res = None;
            {
                let mut i = s.chars();
                loop {
                    match i.next() {
                        None => break,
                        Some(c) => {
                            if c == '\n' {
                                res = Some(i.as_str().to_string());
                                break;
                            } else if !c.is_whitespace() {
                                break;
                            }
                        }
                    }
                }
            }
            if let Some(res) = res {
                *s = res.to_string();
            }
        }
    }

    let mut stripped_items = vec![];

    /* Strip indentation. */
    for (n, item) in doc.iter().enumerate() {
        match item {
            Item::Text(s, pos) => {
                let i = strip_indent(&s, &indent.s, n == 0);
                stripped_items.push(Item::Text(i, pos.clone()))
            }
            _ => stripped_items.push(item.clone()), // FIXME
        }
    }

    Ok((Doc(stripped_items), indent))
}

fn parse_doc2<'a>(state: &mut State, required_end: Option<&str>) -> Result<(Doc, Indent), Error> {
    let mut items = vec![];
    let mut text = String::new();
    let mut text_pos = state.pos();
    let mut indent = Indent::new();

    loop {
        let c = state.peek();

        match c {
            Some('\\') => {
                let pos = state.pos();

                if !text.is_empty() {
                    indent = unify_indents(indent, get_indent(&text));
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
                        Some(required_tag) if tag == required_tag => Ok((Doc(items), indent)),
                        Some(required_tag) => Err(Error::MismatchingTags(
                            required_tag.to_string(),
                            tag,
                            state.pos(),
                        )),
                        None => Err(Error::UnexpectedEnd(state.pos())),
                    };
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
                            let (child, child_indent) = parse_doc(state, None)?;
                            indent = unify_indents(indent, child_indent);
                            named_args.insert(tag, child);
                            state.eat(|c| c == ']')?;
                        }
                        _ => break,
                    }
                }

                let mut pos_args = vec![];
                loop {
                    //skip_ws(state);
                    match state.peek() {
                        Some('{') => {
                            state.next();
                            let (child, child_indent) = parse_doc(state, None)?;
                            indent = unify_indents(indent, child_indent);
                            pos_args.push(child);
                            state.eat(|c| c == '}')?;
                        }
                        _ => break,
                    }
                }

                if have_begin {
                    let (child, child_indent) = parse_doc(state, Some(&tag))?;
                    indent = unify_indents(indent, child_indent);
                    pos_args.push(child);
                }

                items.push(
                    Element {
                        tag,
                        named_args,
                        pos_args,
                        pos,
                    }
                    .into(),
                );
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
                    return Err(Error::MissingEnd(
                        required_end.unwrap().to_string(),
                        state.pos(),
                    ));
                }
                if !text.is_empty() {
                    indent = unify_indents(indent, get_indent(&text));
                    items.push(Item::Text(text, text_pos));
                };
                return Ok((Doc(items), indent));
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
                if c2 == '}' {
                    return Ok(());
                }
                res.push(c);
                res.push(c2);
            }
            _ => res.push(c),
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
            }
            _ => {
                if tag.is_empty() {
                    return Err(Error::TagExpected(state.pos()));
                }
                return Ok(tag);
            }
        }
    }
}

fn parse_regular_tag(state: &mut State) -> Result<Tag, Error> {
    let tag = parse_tag(state)?;
    if tag == "begin" || tag == "end" {
        return Err(Error::InvalidTagName(state.pos()));
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
            _ => return,
        };
    }
}

fn get_indent(s: &str) -> Indent {
    let mut indent = Indent::new();
    let mut indent_start = 0;
    let mut indent_end = 0;
    let mut in_indent = true;

    for (pos, c) in s.char_indices() {
        if c == '\n' {
            if in_indent {
                indent_end = pos;
            }
            let i = Indent {
                open: in_indent,
                s: (&s[indent_start..indent_end]).to_string(),
            };
            indent = unify_indents(indent, i);
            indent_start = pos + 1;
            in_indent = true;
        } else if in_indent {
            if !c.is_whitespace() {
                in_indent = false;
                indent_end = pos;
            }
        }
    }

    indent
}

fn unify_indents(s1: Indent, s2: Indent) -> Indent {
    let mut i1 = s1.s.char_indices();
    let mut i2 = s2.s.chars();

    loop {
        let c1 = i1.next();
        let c2 = i2.next();
        if let Some(c1) = c1 {
            if let Some(c2) = c2 {
                if c1.1 != c2 {
                    return Indent {
                        open: false,
                        s: (&s1.s[0..c1.0]).to_string(),
                    };
                }
            } else {
                return if s2.open { s1.clone() } else { s2.clone() }; // FIXME: don't clone
            }
        } else {
            return if s1.open { s2.clone() } else { s1.clone() }; // FIXME: don't clone
        }
    }
}

fn strip_indent(s: &str, indent: &str, strip_first: bool) -> String {
    let mut res = String::new();
    let mut i = s.chars().peekable();

    loop {
        /* Skip the indentation. */
        if strip_first {
            let mut j = indent.chars();
            loop {
                if let Some(c1) = i.peek() {
                    if let Some(c2) = j.next() {
                        if c1 != &c2 {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    return res;
                }
                i.next();
            }
        }

        /* Copy all characters up to and including the end-of-line. */
        loop {
            if let Some(c) = i.next() {
                res.push(c);
                if c == '\n' {
                    break;
                }
            } else {
                return res;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::Item;

    #[test]
    fn parse_str() {
        assert_eq!(
            parse_string(None, "hello"),
            Ok(Item::Text(
                "hello".to_string(),
                Pos {
                    filename: None,
                    line: 0,
                    column: 0
                }
            )
            .into())
        );
    }

    #[test]
    fn parse_element() {
        assert_eq!(
            parse_string(None, "Hello \\emph{World}!"),
            Ok(Doc(vec![
                Item::Text(
                    "Hello ".to_string(),
                    Pos {
                        filename: None,
                        line: 0,
                        column: 0
                    }
                ),
                Element {
                    tag: "emph".to_string(),
                    named_args: HashMap::new(),
                    pos_args: vec![Doc(vec![Item::Text(
                        "World".to_string(),
                        Pos {
                            filename: None,
                            line: 0,
                            column: 12
                        }
                    )])],
                    pos: Pos {
                        filename: None,
                        line: 0,
                        column: 6
                    }
                }
                .into(),
                Item::Text(
                    "!".to_string(),
                    Pos {
                        filename: None,
                        line: 0,
                        column: 18
                    }
                ),
            ]))
        );
    }

    #[test]
    fn parse_element_eof() {
        assert_eq!(
            parse_string(None, "Hello \\emph{World!"),
            Err(Error::UnexpectedEOF(Pos {
                filename: None,
                line: 0,
                column: 18
            }))
        );
    }

    #[test]
    fn parse_begin_end() {
        assert_eq!(
            parse_string(None, "\\begin{emph}bla\\end{emph}"),
            Ok(Element {
                tag: "emph".to_string(),
                named_args: HashMap::new(),
                pos_args: vec![Doc(vec![Item::Text(
                    "bla".to_string(),
                    Pos {
                        filename: None,
                        line: 0,
                        column: 12
                    }
                )])],
                pos: Pos {
                    filename: None,
                    line: 0,
                    column: 0
                }
            }
            .into())
        );
    }

    #[test]
    fn parse_begin_end_mismatch() {
        assert_eq!(
            parse_string(None, "\\begin{emph}bla\\end{emp}"),
            Err(Error::MismatchingTags(
                "emph".to_string(),
                "emp".to_string(),
                Pos {
                    filename: None,
                    line: 0,
                    column: 24
                }
            ))
        );
    }
}
