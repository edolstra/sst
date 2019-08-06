use serde::Serialize;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
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

impl From<Item> for Doc {
    fn from(item: Item) -> Self {
        Doc(vec![item])
    }
}

impl From<Element> for Doc {
    fn from(item: Element) -> Self {
        Doc(vec![item.into()])
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Item {
    Text { text: String, pos: Pos },
    Element(Element),
}

impl Item {
    pub fn new_text(text: String, pos: Pos) -> Self {
        Item::Text { text, pos }
    }

    pub fn get_text(&self) -> Option<&str> {
        match self {
            Item::Text { text, .. } => Some(text),
            _ => None,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            Item::Text { text, .. } => text.chars().all(char::is_whitespace),
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

impl From<Element> for Item {
    fn from(elem: Element) -> Self {
        Item::Element(elem)
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub tag: Tag,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub named_args: HashMap<String, Doc>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Pos {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<Arc<String>>,
    pub line: u64,
    pub column: u64,
}
