use crate::{ast::*, parser};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    WrongMacroArgCount(String, usize, usize), // FIXME: add Pos
    WrongDefArgCount(usize),
    InvalidMacroName,
    BadArity,
    BadStrip(Pos),
    BadInclude(Pos),
    UnknownBase(Pos),
    IOError(Pos, PathBuf, io::Error),
}

pub fn eval(doc: &Doc) -> Result<Doc, Error> {
    eval2(None, doc)
}

type Env = Option<Rc<Macro>>;

#[derive(Debug)]
struct Macro {
    name: String,
    arity: usize,
    defaults: HashMap<String, Doc>,
    body: Doc,
    next: Env,
}

fn eval2(env: Env, doc: &Doc) -> Result<Doc, Error> {
    let mut items = vec![];
    eval_into(&mut items, env, doc)?;
    Ok(Doc(items))
}

fn eval_into(items: &mut Vec<Item>, mut env: Env, doc: &Doc) -> Result<(), Error> {
    for item in doc.iter() {
        match item {
            Item::Text { text, pos } => append_text(items, text, pos),
            Item::Element(elem) => {
                if elem.tag == "def" {
                    if elem.pos_args.len() != 2 {
                        return Err(Error::WrongDefArgCount(elem.pos_args.len()));
                    }
                    env = Some(Rc::new(Macro {
                        name: elem.pos_args[0][0]
                            .get_text()
                            .ok_or(Error::InvalidMacroName)?
                            .to_string(),
                        arity: match elem.named_args.get("arity") {
                            None => 0,
                            Some(x) => get_text(&x)
                                .ok_or(Error::BadArity)?
                                .parse()
                                .map_err(|_| Error::BadArity)?,
                        },
                        defaults: elem.named_args.clone(),
                        body: elem.pos_args[1].clone(),
                        next: env.clone(),
                    }));
                } else if elem.tag == "#" {
                } else if elem.tag == "strip" {
                    if elem.pos_args.len() != 1 {
                        return Err(Error::BadStrip(elem.pos.clone()));
                    }
                    eval_into(items, env.clone(), &elem.pos_args[0])?;
                } else if elem.tag == "include" {
                    let (filename, file) = read_file_from(&elem)?;
                    let ast = parser::parse_string(Some(&filename), &file).expect("Parse error");
                    eval_into(items, None, &ast)?;
                } else if elem.tag == "includeraw" {
                    let (filename, file) = read_file_from(&elem)?;
                    append_text(
                        items,
                        &file,
                        &Pos {
                            filename: Some(Arc::new(filename)),
                            line: 0,
                            column: 0,
                        },
                    );
                } else {
                    if let Some(m) = lookup_env(&elem.tag, &env) {
                        let mut env = m.next.clone();

                        if m.arity != elem.pos_args.len() && !(m.arity == 0 && elem.is_empty()) {
                            return Err(Error::WrongMacroArgCount(
                                m.name.clone(),
                                m.arity,
                                elem.pos_args.len(),
                            ));
                        }

                        for (name, def) in &m.defaults {
                            match elem.named_args.get(name) {
                                None => env = to_macro(name.to_string(), &def, &env),
                                Some(arg) => env = to_macro(name.to_string(), &arg, &env),
                            }
                        }

                        for n in 0..m.arity {
                            env = to_macro(n.to_string(), &elem.pos_args[n], &env);
                        }

                        eval_into(items, env.clone(), &m.body)?;
                    } else {
                        let mut named_args = HashMap::new();
                        for (name, body) in &elem.named_args {
                            named_args.insert(name.clone(), eval2(env.clone(), &body)?);
                        }
                        let mut pos_args = vec![];
                        for arg in &elem.pos_args {
                            pos_args.push(eval2(env.clone(), &arg)?);
                        }
                        items.push(Item::Element(Element {
                            tag: elem.tag.clone(),
                            named_args,
                            pos_args,
                            pos: elem.pos.clone(),
                        }));
                    }
                }
            }
        }
    }

    Ok(())
}

fn append_text(items: &mut Vec<Item>, s2: &str, p2: &Pos) {
    if let Some(Item::Text { text, .. }) = items.last_mut() {
        text.push_str(s2);
        return;
    }
    items.push(Item::new_text(s2.to_string(), p2.clone()))
}

fn lookup_env(name: &str, mut env: &Env) -> Env {
    loop {
        env = match &env {
            Some(ref m) if m.name != name => &m.next,
            _ => return env.clone(),
        }
    }
}

fn to_macro(name: String, body: &Doc, env: &Env) -> Env {
    Some(Rc::new(Macro {
        name,
        arity: 0,
        defaults: HashMap::new(),
        body: body.clone(),
        next: env.clone(), // FIXME?
    }))
}

fn get_text<'a>(arg: &'a Doc) -> Option<&'a str> {
    if arg.len() != 1 {
        None
    } else {
        match &arg[0] {
            Item::Text { text, .. } => Some(&text),
            _ => None,
        }
    }
}

fn read_file_from(elem: &Element) -> Result<(PathBuf, String), Error> {
    if elem.pos_args.len() != 1 {
        return Err(Error::BadInclude(elem.pos.clone()));
    }
    let filename =
        get_text(&elem.pos_args[0]).ok_or_else(|| Error::BadInclude(elem.pos.clone()))?;
    if let Some(parent_filename) = &elem.pos.filename {
        let path = Path::new(&**parent_filename)
            .parent()
            .unwrap()
            .join(&filename);
        match fs::read_to_string(&path) {
            Ok(s) => Ok((path, s)),
            Err(err) => Err(Error::IOError(elem.pos.clone(), path, err)),
        }
    } else {
        // FIXME: we don't need parent_filename if filename is absolute.
        Err(Error::UnknownBase(elem.pos.clone()))
    }
}
