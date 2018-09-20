use std::collections::HashMap;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Doc(pub Vec<Item>);

impl Deref for Doc {
    type Target = Vec<Item>;
    fn deref(&self) -> &Vec<Item> { &self.0 }
}

#[derive(Debug, Clone)]
pub enum Item {
    Text(String, Pos),
    Element(Element),
}

impl Item {
    pub fn is_whitespace(&self) -> bool {
        match self {
            Item::Text(s, _) => s.chars().all(char::is_whitespace),
            Item::Element(_) => false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: Tag,
    pub named_args: HashMap<String, Doc>,
    pub pos_args: Vec<Doc>,
    pub pos: Pos,
}

pub type Tag = String;

#[derive(Debug, Clone)]
pub struct Pos {
    pub filename: String, // FIXME: wasteful
    pub line: u64,
    pub column: u64,
}
