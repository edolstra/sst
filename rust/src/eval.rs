use ast::*;
use parser;
use std::collections::HashMap;
use std::rc::Rc;
use std::fs;
use std::path::Path;
use std::io;

#[derive(Debug)]
pub enum Error {
    WrongMacroArgCount(String, usize, usize), // FIXME: add Pos
    WrongDefArgCount(usize),
    InvalidMacroName,
    BadArity,
    BadStrip(Pos),
    BadInclude(Pos),
    IOError(Pos, String, io::Error),
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
            Item::Text(s2, pos) => append_text(items, s2, pos),
            Item::Element(elem) => {
                if elem.tag == "def" {
                    if elem.pos_args.len() != 2 {
                        return Err(Error::WrongDefArgCount(elem.pos_args.len()))
                    }
                    env = Some(Rc::new(Macro {
                        name: match &elem.pos_args[0][0] { // FIXME
                            Item::Text(name, _) => name.clone(),
                            _ => return Err(Error::InvalidMacroName)
                        },
                        arity: match elem.named_args.get("arity") {
                            None => 0,
                            Some(x) => {
                                match get_text(&x, Error::BadArity)?.parse() {
                                    Ok(n) => n,
                                    Err(_) => return Err(Error::BadArity)
                                }
                            }
                        },
                        defaults: elem.named_args.clone(),
                        body: elem.pos_args[1].clone(),
                        next: env.clone(),
                    }));
                }

                else if elem.tag == "#" { }

                else if elem.tag == "strip" {
                    if elem.pos_args.len() != 1 { return Err(Error::BadStrip(elem.pos.clone())); }
                    eval_into(items, env.clone(), &elem.pos_args[0])?;
                }

                else if elem.tag == "include" {
                    let (filename, file) = read_file_from(&elem)?;
                    let ast = parser::parse_string(&filename, &file).expect("Parse error");
                    eval_into(items, None, &ast)?;
                }

                else if elem.tag == "includeraw" {
                    let (filename, file) = read_file_from(&elem)?;
                    append_text(items, &file, &Pos { filename, line: 0, column: 0 });
                }

                else {

                    if let Some(m) = lookup_env(&elem.tag, &env) {
                        let mut env = m.next.clone();

                        if m.arity != elem.pos_args.len() && !(m.arity == 0 && elem.is_empty()) {
                            return Err(Error::WrongMacroArgCount(m.name.clone(), m.arity, elem.pos_args.len()));
                        }

                        for (name, def) in &m.defaults {
                            match elem.named_args.get(name) {
                                None => env = to_macro(name.to_string(), &def, &env),
                                Some(arg) => env = to_macro(name.to_string(), &arg, &env)
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
                            named_args, pos_args,
                            pos: elem.pos.clone()
                        }));
                    }
                }
            }
        }
    }

    Ok(())
}

fn append_text(items: &mut Vec<Item>, s2: &str, p2: &Pos) {
    if let Some(Item::Text(s, _)) = items.last_mut() {
        s.push_str(s2);
        return
    }
    items.push(Item::Text(s2.to_string(), p2.clone()))
}

fn lookup_env(name: &str, mut env: &Env) -> Env {
    loop {
        env = match &env {
            Some(ref m) if m.name != name => &m.next,
            _ => return env.clone()
        }
    }
}

fn to_macro(name: String, body: &Doc, env: &Env) -> Env {
    Some(Rc::new(Macro {
        name,
        arity: 0,
        defaults: HashMap::new(),
        body: body.clone(),
        next: env.clone() // FIXME?
    }))
}

fn get_text<'a>(arg: &'a Doc, err: Error) -> Result<&'a str, Error> {
    if arg.len() != 1 { return Err(err) }
    match &arg[0] {
        Item::Text(s, _) => return Ok(&s),
        _ => return Err(err)
    }
}

fn read_file_from(elem: &Element) -> Result<(String, String), Error> {
    if elem.pos_args.len() != 1 { return Err(Error::BadInclude(elem.pos.clone())); }
    let filename = get_text(&elem.pos_args[0], Error::BadInclude(elem.pos.clone()))?;
    let path = Path::new(&elem.pos.filename).parent().unwrap().join(&filename);
    let filename = path.to_str().unwrap();
    match fs::read_to_string(&filename) {
        Ok(s) => Ok((filename.to_string(), s)),
        Err(err) => Err(Error::IOError(elem.pos.clone(), filename.to_string(), err))
    }
}
