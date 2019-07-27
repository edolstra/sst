use serde::Serialize;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Serialize, Debug, Clone)]
pub struct Doc(pub Vec<Item>);

impl Deref for Doc {
    type Target = Vec<Item>;
    fn deref(&self) -> &Vec<Item> {
        &self.0
    }
}

impl DerefMut for Doc {
    fn deref_mut(&mut self) -> &mut Vec<Item> {
        &mut self.0
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum Item {
    Text(String, Pos),
    Element(Element),
}

impl Item {
    pub fn is_whitespace(&self) -> bool {
        match self {
            Item::Text(s, _) => s.chars().all(char::is_whitespace),
            Item::Element(_) => false,
        }
    }

    /*
    pub fn get_pos(&self) -> &Pos {
        match self {
            Item::Text(_, pos) => pos,
            Item::Element(element) => &element.pos
        }
    }
    */
}

#[derive(Serialize, Debug, Clone)]
pub struct Element {
    pub tag: Tag,
    pub named_args: HashMap<String, Doc>,
    pub pos_args: Vec<Doc>,
    pub pos: Pos,
}

impl Element {
    /// Test whether the element is empty, i.e. has only one empty
    /// argument (e.g. \foo{}).
    pub fn is_empty(&self) -> bool {
        self.pos_args.len() == 0 || (self.pos_args.len() == 1 && self.pos_args[0].is_empty())
    }
}

pub type Tag = String;

#[derive(Serialize, Debug, Clone)]
pub struct Pos {
    pub filename: String, // FIXME: wasteful
    pub line: u64,
    pub column: u64,
}
