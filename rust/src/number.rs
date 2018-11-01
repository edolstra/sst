use validate::Instance;
use std::collections::HashMap;
use std::rc::Rc;
use std::hash::{Hash, Hasher};

pub struct InstanceByAddr<'a>(pub &'a Instance);

impl<'a> Hash for InstanceByAddr<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 as *const Instance).hash(state)
    }
}

impl<'a> Eq for InstanceByAddr<'a> {
}

impl<'a> PartialEq for InstanceByAddr<'a> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 as *const Instance).eq(&(other.0 as *const Instance))
    }
}

pub struct Numbers<'doc> {
    pub toc: HashMap<InstanceByAddr<'doc>, Rc<TocEntry<'doc>>>,
}

pub struct TocEntry<'doc> {
    pub parent: Option<Rc<TocEntry<'doc>>>,
    pub number: usize,
    pub title: &'doc Instance,
}

impl<'doc> TocEntry<'doc> {
    pub fn get_path(&self) -> Vec<String> {
        let mut numbers = vec![];
        let mut entry = self;
        loop {
            numbers.push(entry.number.to_string());
            if let Some(parent) = &entry.parent { entry = &*parent; } else { break }
        }
        numbers.reverse();
        numbers
    }

    pub fn to_string(&self) -> String {
        self.get_path().join(".")
    }
}

impl<'doc> Numbers<'doc> {

    pub fn create(doc: &'doc Instance) -> Self {
        let mut numbers = Numbers {
            toc: HashMap::new(),
        };
        let mut next_number: usize = 1;
        numbers.traverse(doc, None, &mut next_number);
        numbers
    }

    pub fn get_toc_entry(&self, doc: &'doc Instance) -> Option<&TocEntry<'doc>> {
        match self.toc.get(&InstanceByAddr(doc)) {
            None => None,
            Some(x) => Some(&*x)
        }
    }

    fn traverse(&mut self, doc: &'doc Instance,
                parent: Option<Rc<TocEntry<'doc>>>,
                next_number: &mut usize)
    {
        let mut parent = parent;
        let mut new_counter: usize = 1;
        let mut next_number = next_number;

        match doc {
            Instance::Element(tag, children) if tag == "chapter" || tag == "section" || tag == "subsection" => {
                let entry = Rc::new(TocEntry {
                    parent: parent.clone(),
                    number: *next_number,
                    title: &children[0]
                });
                self.toc.insert(InstanceByAddr(doc), entry.clone());
                parent = Some(entry);
                *next_number += 1;
                next_number = &mut new_counter;
            },
            _ => {}
        }

        match doc {
            Instance::Text(_) => {},
            Instance::Element(_, children) => {
                for child in children.iter() {
                    self.traverse(child, parent.clone(), next_number);
                }
            },
            Instance::Para(child) => self.traverse(child, parent.clone(), next_number),
            Instance::Seq(children) => {
                for child in children.iter() {
                    self.traverse(child, parent.clone(), next_number);
                }
            }
            Instance::Choice(_, child) => self.traverse(child, parent.clone(), next_number),
            Instance::Many(children) => {
                for child in children.iter() {
                    self.traverse(child, parent.clone(), next_number);
                }
            }
        }
    }

}
