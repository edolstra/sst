use crate::ast::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, none_of, one_of},
    combinator::{all_consuming, cut, map},
    error::ErrorKind,
    multi::{many0, many1},
    sequence::{preceded, tuple},
    IResult,
};
use nom_locate::LocatedSpanEx;
use std::iter::Peekable;
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

impl<'a> nom::error::ParseError<Span<'a>> for Error {
    fn from_error_kind(input: Span<'a>, _kind: ErrorKind) -> Self {
        if let Some(c) = input.fragment.chars().next() {
            Error::UnexpectedChar(c, (&input).into())
        } else {
            Error::UnexpectedEOF((&input).into())
        }
    }

    fn append(_input: Span<'a>, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

type Span<'a> = LocatedSpanEx<&'a str, &'a Option<Filename>>;

type PResult<'a, T> = IResult<Span<'a>, T, Error>;

impl<'a> From<&Span<'a>> for Pos {
    fn from(span: &Span<'a>) -> Self {
        Pos {
            filename: span.extra.clone(),
            line: span.line - 1,
            column: span.get_utf8_column() as u32 - 1,
        }
    }
}

pub fn text<'a>(input: Span<'a>) -> PResult<Item> {
    let text_char = none_of("{}[]\\");
    map(many1(text_char), |cs| {
        Item::new_text(cs.into_iter().collect(), (&input).into())
    })(input)
}

pub fn raw<'a>(input: Span<'a>) -> PResult<Item> {
    // FIXME: support nesting
    map(
        tuple((tag("{{"), many0(none_of("{}")), tag("}}"))),
        |(_, cs, _)| Item::new_text(cs.into_iter().collect(), (&input).into()),
    )(input)
}

pub fn tag_name<'a>() -> impl Fn(Span<'a>) -> PResult<String> {
    map(
        many1(one_of("abcdefghijklmnopqrstuvwxyz0123456789#")),
        |cs| cs.into_iter().collect::<String>(),
    )
}

pub fn named_arg<'a>() -> impl Fn(Span<'a>) -> PResult<(String, Doc)> {
    // FIXME: whitespace
    preceded(
        char('['),
        cut(map(
            tuple((tag_name(), char('='), doc, char(']'))),
            |(tag, _, doc, _)| (tag, doc),
        )),
    )
}

pub fn pos_arg<'a>() -> impl Fn(Span<'a>) -> PResult<Doc> {
    preceded(char('{'), cut(map(tuple((doc, char('}'))), |(doc, _)| doc)))
}

pub fn element<'a>(input: Span<'a>) -> PResult<Item> {
    let (rest, (_, (tag, named_args, pos_args))) = tuple((
        char('\\'),
        cut(tuple((tag_name(), many0(named_arg()), many1(pos_arg())))),
    ))(input)?;

    if tag == "begin" || tag == "end" {
        Err(nom::Err::Error(Error::InvalidTagName((&rest).into())))
    } else {
        Ok((
            rest,
            Element {
                tag,
                named_args: named_args.into_iter().collect(),
                pos_args,
                pos: (&input).into(),
            }
            .into(),
        ))
    }
}

pub fn long_element<'a>(input: Span<'a>) -> PResult<Item> {
    let (rest, (_, (open_tag, _, named_args, mut pos_args, doc))) = tuple((
        tag("\\begin{"),
        cut(tuple((
            tag_name(),
            char('}'),
            many0(named_arg()),
            many0(pos_arg()),
            doc,
        ))),
    ))(input)?;

    let (rest, (_, close_tag, _)) =
        tuple((tag("\\end{"), tag_name(), char('}')))(rest).map_err(|err| {
            if let nom::Err::Error(Error::UnexpectedEOF(pos)) = err {
                nom::Err::Failure(Error::MissingEnd(open_tag.to_string(), pos))
            } else {
                err
            }
        })?;

    if open_tag == close_tag {
        if open_tag == "begin" || open_tag == "end" {
            Err(nom::Err::Failure(Error::InvalidTagName((&input).into())))
        } else {
            pos_args.push(doc);
            Ok((
                rest,
                Element {
                    tag: open_tag,
                    named_args: named_args.into_iter().collect(),
                    pos_args,
                    pos: (&input).into(),
                }
                .into(),
            ))
        }
    } else {
        Err(nom::Err::Failure(Error::MismatchingTags(
            open_tag,
            close_tag,
            (&rest).into(),
        )))
    }
}

pub fn doc<'a>(input: Span<'a>) -> PResult<Doc> {
    let item = alt((text, raw, element, long_element));
    map(many0(item), |items| Doc(items))(input)
}

pub fn parse_string(filename: Option<&str>, s: &str) -> Result<Doc, Error> {
    let filename = filename.map(|filename| Arc::new(filename.to_string()));
    let input = Span::new_extra(s, &filename);

    let res = all_consuming(doc)(input);

    match res {
        Err(nom::Err::Error(err)) => unreachable!("ERROR {:?}", err),
        Err(nom::Err::Failure(err)) => Err(err),
        Err(nom::Err::Incomplete(_)) => unreachable!(),
        Ok((_, s)) => Ok(s),
    }
}

/*
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
        if let Item::Text { ref mut text, .. } = doc[0] {
            let mut res = None;
            {
                let mut i = text.chars();
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
                *text = res.to_string();
            }
        }
    }

    let mut stripped_items = vec![];

    /* Strip indentation. */
    for (n, item) in doc.iter().enumerate() {
        match item {
            Item::Text { text, pos } => {
                let i = strip_indent(&text, &indent.s, n == 0);
                stripped_items.push(Item::new_text(i, pos.clone()))
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
                    items.push(Item::new_text(
                        mem::replace(&mut text, String::new()),
                        text_pos,
                    ));
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
                    items.push(Item::new_text(text, text_pos));
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
*/

#[cfg(test)]
mod test {
    use super::*;

    fn check_ok(sst: &str, json: &str) {
        assert_eq!(
            serde_json::to_string_pretty(&parse_string(None, sst).unwrap()).unwrap() + "\n",
            json
        );
    }

    fn check_err(sst: &str, err: Error) {
        assert_eq!(parse_string(None, sst), Err(err));
    }

    #[test]
    fn parse_text() {
        check_ok(
            include_str!("../../test/text.sst"),
            include_str!("../../test/text.json"),
        );
    }

    #[test]
    fn parse_element() {
        check_ok(
            include_str!("../../test/element.sst"),
            include_str!("../../test/element.json"),
        );
    }

    #[test]
    fn parse_element_eof() {
        check_err(
            include_str!("../../test/element-eof.sst"),
            Error::UnexpectedEOF(Pos {
                filename: None,
                line: 0,
                column: 18,
            }),
        )
    }

    #[test]
    fn parse_begin_end() {
        check_ok(
            include_str!("../../test/long-element.sst"),
            include_str!("../../test/long-element.json"),
        );
    }

    #[test]
    fn parse_begin_end_mismatch() {
        check_err(
            include_str!("../../test/long-element-mismatch.sst"),
            Error::MismatchingTags(
                "emph".to_string(),
                "emp".to_string(),
                Pos {
                    filename: None,
                    line: 0,
                    column: 24,
                },
            ),
        );
    }

    #[test]
    fn parse_indent() {
        check_ok(
            include_str!("../../test/indent.sst"),
            include_str!("../../test/indent.json"),
        );
    }
}
