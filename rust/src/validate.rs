use ast::*;
use schema::*;
use std::mem;
use std::str::Chars;

#[derive(Debug, Clone)]
pub enum Error {
    Expected(Vec<Expected>, Pos),
    WrongArgCount(Tag, usize, usize, Pos),
    WrongElementContent(Tag, Pos, Box<Error>),
    SchemaError(Tag),
}

#[derive(Debug, Clone)]
pub enum Expected {
    Text,
    Para,
    Element(Tag),
    End,
}

impl Error {
    fn is_fatal(&self) -> bool {
        match self {
            Error::WrongArgCount(_, _, _, _) => true,
            Error::WrongElementContent(_, _, _) => true,
            Error::SchemaError(_) => true,
            _ => false
        }
    }
}

#[derive(Serialize, Debug)]
pub enum Instance {
    Text(String),
    Element(Tag, Vec<Instance>),
    Para(Box<Instance>),
    Seq(Vec<Instance>),
    Choice(usize, Box<Instance>),
    Many(Vec<Instance>),
}

impl Instance {
    pub fn unchoice(&self) -> &Self {
        match self {
            Instance::Choice(_, i) => &i,
            _ => &self
        }
    }

    pub fn seq(&self) -> &Vec<Instance> {
        match self {
            Instance::Seq(is) => is,
            _ => panic!()
        }
    }

    pub fn many(&self) -> &Vec<Instance> {
        match self {
            Instance::Many(is) => is,
            _ => panic!()
        }
    }

    pub fn iter(&self) -> &Vec<Instance> {
        match self {
            Instance::Many(is) => is,
            _ => panic!()
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            Instance::Text(s) => s.chars().all(char::is_whitespace),
            Instance::Element(_, _) => false,
            Instance::Para(i) => i.is_whitespace(),
            Instance::Seq(is) => is.iter().all(Instance::is_whitespace),
            Instance::Choice(_, i) => i.is_whitespace(),
            Instance::Many(is) => is.iter().all(Instance::is_whitespace),
        }
    }
}

pub fn validate(schema: &Schema, doc: &Doc, filename: &str) -> Result<Instance, Error> {
    validate_full_doc(schema, &schema.start, doc,
                      Pos { filename: filename.to_string(), line: 0, column: 0 })
}

#[derive(Clone)]
struct Cursor<'a> {
    items: &'a [Item],
    pending_chars: Chars<'a>,
    in_para: ParaState,
    cur_pos: Pos,
}

#[derive(PartialEq, Eq, Clone)]
enum ParaState { No, Start, Inside, End }

impl<'a> Cursor<'a> {
    fn new(items: &'a [Item], pos: Pos) -> Self {
         Cursor { items, pending_chars: "".chars(), in_para: ParaState::No, cur_pos: pos }
    }

    fn pos(&self) -> Pos {
        /*
        match self.items.first() {
            Some(Item::Element(element)) => element.pos.clone(),
            Some(Item::Text(_, pos)) => Some(pos.clone()),
            _ => self.cur_pos.clone(),
        }
         */
        self.cur_pos.clone()
    }

    fn peek_char(&mut self) -> Option<char> {
        if self.in_para == ParaState::End { None }
        else if let Some(c) = self.pending_chars.clone().next() { Some(c) }
        else if let Some(Item::Text(s, _)) = self.items.first() { s.chars().next() }
        else { None }
    }

    fn get_char(&mut self) -> Option<char> {
        if self.in_para == ParaState::End { None }

        else if let Some(c) = self.pending_chars.next() {
            if c == '\n' {
                self.cur_pos.line += 1;
                self.cur_pos.column = 0;
            } else {
                self.cur_pos.column += 1;
            }
            Some(c)
        }

        else if let Some(Item::Text(s, pos)) = self.items.first() {
            self.cur_pos = pos.clone();
            self.items = &self.items[1..];
            self.pending_chars = s.chars();
            self.pending_chars.next()
        }

        else { None}
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() { break };
            self.get_char();
        }
    }

    fn get_element(&mut self, tag: &Tag) -> Option<&'a Element> {
        if !self.pending_chars.clone().all(char::is_whitespace) { return None; }

        let mut items = self.items;
        while let Some(item) = items.first() {
            if !item.is_whitespace() { break; }
            items = &items[1..];
        }

        match items.first() {
            Some(Item::Element(element)) if &element.tag == tag => {
                self.pending_chars = "".chars();
                self.items = &items[1..];
                self.cur_pos = element.pos.clone();
                Some(element)
            }
            _ => None
        }
    }

    fn at_end(&self) -> bool {
        self.in_para == ParaState::End || (self.items.is_empty() && self.pending_chars.as_str().is_empty())
    }

    fn at_end_ws(&self) -> bool {
        let mut c = self.clone();
        c.skip_ws();
        c.at_end()
    }
}

pub fn validate_full_doc(schema: &Schema, pattern: &Pattern, doc: &Doc, pos: Pos) -> Result<Instance, Error> {
    let mut cursor = Cursor::new(doc, pos);
    let instance = validate_doc(schema, pattern, true, &mut cursor)?;
    cursor.skip_ws();
    if !cursor.at_end() {
        return Err(Error::Expected(vec![Expected::End], cursor.pos()))
    }
    Ok(instance)
}

fn validate_doc(schema: &Schema, pattern: &Pattern, at_top: bool, mut cursor: &mut Cursor) -> Result<Instance, Error> {
    match pattern {

        Pattern::Text => {
            let mut text = String::new();
            let mut in_empty_line = false;
            while let Some(c) = cursor.get_char() {
                text.push(c);
                match cursor.in_para {
                    ParaState::No => {},
                    ParaState::Start => {
                        if !c.is_whitespace() {
                            cursor.in_para = ParaState::Inside;
                        }
                    }
                    ParaState::Inside => {
                        if c == '\n' {
                            if in_empty_line {
                                cursor.in_para = ParaState::End;
                                break;
                            }
                            in_empty_line = true;
                        } else if in_empty_line && !c.is_whitespace() {
                            in_empty_line = false;
                        }
                    }
                    ParaState::End => panic!()
                }
            }
            if text.is_empty() {
                return Err(Error::Expected(vec![Expected::Text], cursor.pos()));
            }
            return Ok(Instance::Text(text));
        }

        Pattern::Para(pat) => {
            assert!(cursor.in_para == ParaState::No);
            if cursor.at_end_ws() {
                return Err(Error::Expected(vec![Expected::Para], cursor.pos()));
            }
            cursor.in_para = ParaState::Start;
            let instance = validate_doc(schema, pat, false, cursor)?;
            assert!(cursor.in_para != ParaState::No);
            cursor.in_para = ParaState::No;
            if instance.is_whitespace() {
                return Err(Error::Expected(vec![Expected::Para], cursor.pos()));
            } else {
                return Ok(Instance::Para(Box::new(instance)));
            }
        }

        Pattern::Element(name) => {
            if let Some(pos_args_patterns) = schema.elements.get(name) {
                if let Some(element) = cursor.get_element(name) {
                    let mut instances = vec!();
                    if (pos_args_patterns.len() == 0 && !element.is_empty())
                        || (pos_args_patterns.len() > 0 && pos_args_patterns.len() != element.pos_args.len())
                    {
                        return Err(Error::WrongArgCount(
                            name.clone(),
                            pos_args_patterns.len(),
                            element.pos_args.len(),
                            cursor.pos()));
                    }
                    for (d, e) in pos_args_patterns.iter().zip(element.pos_args.iter()) {
                        match validate_full_doc(schema, d, e, element.pos.clone()) {
                            Ok(instance) => instances.push(instance),
                            Err(err) => if err.is_fatal() {
                                return Err(err);
                            } else {
                                return Err(Error::WrongElementContent(name.to_string(), element.pos.clone(), Box::new(err)));
                            }
                        };
                    }
                    return Ok(Instance::Element(name.clone(), instances));
                } else {
                    return Err(Error::Expected(vec![Expected::Element(name.clone())], cursor.pos()));
                }
            } else {
                return Err(Error::SchemaError(name.clone()));
            }
        }

        Pattern::Seq(patterns) => {
            let mut instances = vec!();
            for (n, pat) in patterns.iter().enumerate() {
                instances.push(validate_doc(schema, pat, patterns.len() == n + 1 && at_top, &mut cursor)?);
            }
            return Ok(Instance::Seq(instances));
        }

        Pattern::Choice(patterns) => {
            let mut expected = vec!();
            let pos = cursor.pos();
            for (n, pat) in patterns.iter().enumerate() {
                let mut c = cursor.clone();
                match validate_doc(schema, pat, at_top, &mut c) {
                    Ok(instance) => {
                        mem::replace(cursor, c); // FIXME
                        return Ok(Instance::Choice(n, Box::new(instance)));
                    },
                    Err(err) => {
                        if err.is_fatal() {
                            return Err(err);
                        } else {
                            match err {
                                Error::Expected(mut exp, _) => expected.append(&mut exp),
                                _ => return Err(err)
                            }
                        }
                    }
                }
            }
            return Err(Error::Expected(expected, pos));
        }

        Pattern::Many(min, max, pattern) => {
            let mut instances = vec!();
            let mut done = false;
            while !done && (max.is_none() || instances.len() < max.unwrap()) {
                match validate_doc(schema, pattern, false, &mut cursor) {
                    Ok(instance) => {
                        instances.push(instance);
                    }
                    Err(err) => {
                        if err.is_fatal() || instances.len() < *min || (at_top && !cursor.at_end_ws()) { return Err(err); }
                        done = true;
                    }
                }
            }
            return Ok(Instance::Many(instances));
        }
    }
}

