use ast::*;
use schema::*;
use std::mem;

#[derive(Debug)]
pub enum Error {
    Bad,
    NotImplemented(Pattern, Option<Pos>),
    ExpectedText(Option<Pos>),
    ExpectedElement(Tag, Option<Pos>),
    WrongArgCount(Tag, usize, usize, Option<Pos>),
    TrailingContent(Option<Pos>),
}

pub fn validate(schema: &Schema, doc: &Doc) -> Result<(), Error> {
    validate_full_doc(schema, &schema.start, doc)
}

#[derive(Clone)]
struct Cursor<'a> {
    items: &'a [Item],
}

impl<'a> Cursor<'a> {
    fn advance(&mut self) {
        self.items = &self.items[1..];
    }

    fn pos(&self) -> Option<Pos> {
        match self.items.first() {
            Some(Item::Element(element)) => Some(element.pos.clone()),
            Some(Item::Text(_, pos)) => Some(pos.clone()),
            _ => None
        }
    }
}

pub fn validate_full_doc(schema: &Schema, pattern: &Pattern, doc: &Doc) -> Result<(), Error> {
    let mut cursor = Cursor { items: doc };
    validate_doc(schema, pattern, &mut cursor)?;
    skip_ws(&mut cursor);
    if !cursor.items.is_empty() {
        return Err(Error::TrailingContent(cursor.pos()))
    }
    Ok(())
}

fn validate_doc(schema: &Schema, pattern: &Pattern, mut cursor: &mut Cursor) -> Result<(), Error> {

    println!("AT {:?}", cursor.pos());
    match pattern {

        Pattern::Text => {
            match cursor.items.first() {
                Some(Item::Text(_, _)) => { cursor.advance() },
                _ => return Err(Error::ExpectedText(cursor.pos()))
            }
        }

        Pattern::Para(pat) => {
        }

        Pattern::Element(name) => {
            skip_ws(&mut cursor);
            match cursor.items.first() {
                Some(Item::Element(element)) if &element.tag == name => {
                    cursor.advance();
                    let definition = &schema.elements[name];
                    if definition.pos_args.len() != element.pos_args.len() {
                        return Err(Error::WrongArgCount(
                            name.clone(),
                            definition.pos_args.len(),
                            element.pos_args.len(),
                            cursor.pos()))
                    }
                    for (d, e) in definition.pos_args.iter().zip(element.pos_args.iter()) {
                        validate_full_doc(schema, d, e)?
                    }
                },
                _ => return Err(Error::ExpectedElement(name.clone(), cursor.pos()))
            }
        }

        Pattern::Seq(patterns) => {
            for pat in patterns {
                validate_doc(schema, pat, &mut cursor)?
            }
        }

        Pattern::Choice(patterns) => {
            for (n, pat) in patterns.iter().enumerate() {
                let mut c = cursor.clone();
                match validate_doc(schema, pat, &mut c) {
                    Ok(_) => {
                        mem::replace(cursor, c); // FIXME
                        break
                    },
                    Err(err) => if n + 1 == patterns.len() { return Err(err) }
                }
            }
        }

        // FIXME: implement min/max
        Pattern::Many(_, _, pattern) => {
            while !cursor.items.is_empty() {
                validate_doc(schema, pattern, &mut cursor)?
            }
        }

        _ => panic!(format!("{:?}", Error::NotImplemented(pattern.clone(), cursor.pos())))
    }

    Ok(())
}

fn skip_ws(cursor: &mut Cursor) {
    while !cursor.items.is_empty() && cursor.items[0].is_whitespace() {
        cursor.advance();
    }
}

