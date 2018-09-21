use std::collections::HashMap;
use ast::Tag;

pub struct Schema {
    pub start: Pattern,
    pub elements: HashMap<Tag, ElementType>
}

type ElementType = Vec<Pattern>;

impl Schema {
    pub fn add_element(&mut self, name: &str, pos_args: Vec<Pattern>) {
        self.elements.insert(name.to_string(), pos_args);
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Text,
    Element(Tag),
    Para(Box<Pattern>),
    Seq(Vec<Pattern>),
    Choice(Vec<Pattern>),
    Many(usize, Option<usize>, Box<Pattern>),
}

impl Pattern {
    pub fn element(name: &str) -> Self {
        Pattern::Element(name.to_string())
    }

    pub fn para(pattern: Pattern) -> Self {
        Pattern::Para(Box::new(pattern))
    }

    pub fn many(pattern: Pattern) -> Self {
        Pattern::Many(0, None, Box::new(pattern))
    }

    pub fn many1(pattern: Pattern) -> Self {
        Pattern::Many(1, None, Box::new(pattern))
    }
}
