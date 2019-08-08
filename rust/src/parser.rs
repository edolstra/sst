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
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedChar(char, Pos),
    UnexpectedEOF(Pos),
    UnexpectedEnd(Pos),
    MismatchingTags(String, String, Pos), // FIXME: record position of open tag
    MissingEnd(String, Pos),              // FIXME: record name and position of open tag
    TagExpected(Pos),
    InvalidTagName(Pos),
}

impl<'a> nom::error::ParseError<Span<'a>> for Error {
    fn from_error_kind(input: Span<'a>, _kind: ErrorKind) -> Self {
        if let Some(c) = input.fragment.chars().next() {
            // FIXME: delay constructing Pos here.
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
    preceded(
        tag("{{"),
        cut(map(tuple((many0(none_of("{}")), tag("}}"))), |(cs, _)| {
            Item::new_text(cs.into_iter().collect(), (&input).into())
        })),
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
    map(many0(item), |items| Doc(concat_texts(items)))(input)
}

pub fn parse_string(filename: Option<&Path>, s: &str) -> Result<Doc, Error> {
    let filename = filename.map(|filename| Arc::new(filename.into()));
    let input = Span::new_extra(s, &filename);

    let res = all_consuming(doc)(input);

    match res {
        Err(nom::Err::Error(err)) => unreachable!("ERROR {:?}", err),
        Err(nom::Err::Failure(err)) => Err(err),
        Err(nom::Err::Incomplete(_)) => unreachable!(),
        Ok((_, doc)) => Ok(crate::unindent::strip_common_indent(doc)),
    }
}

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
