use crate::ast::{Doc, Item};

pub fn strip_common_indent(doc: Doc) -> Doc {
    strip_common_indent2(doc).0
}

fn strip_common_indent2(mut doc: Doc) -> (Doc, Indent) {
    let mut indent = Indent::new();

    for item in doc.0.iter_mut() {
        match item {
            Item::Text { text, .. } => {
                indent = unify_indents(indent, get_indent(&text));
            }
            Item::Element(element) => {
                for (_, arg) in element.named_args.iter_mut() {
                    let old = std::mem::replace(arg, Doc(vec![]));
                    let (new, arg_indent) = strip_common_indent2(old);
                    std::mem::replace(arg, new);
                    indent = unify_indents(indent, arg_indent);
                }

                for arg in element.pos_args.iter_mut() {
                    let old = std::mem::replace(arg, Doc(vec![]));
                    let (new, arg_indent) = strip_common_indent2(old);
                    std::mem::replace(arg, new);
                    indent = unify_indents(indent, arg_indent);
                }
            }
        }
    }

    /* Strip leading empty line. */
    if !doc.0.is_empty() {
        if let Item::Text { ref mut text, .. } = doc.0[0] {
            let mut res = None;
            {
                let mut i = text.chars();
                loop {
                    match i.next() {
                        None => break,
                        Some(c) => {
                            if c == '\n' {
                                res = Some(i.as_str().to_string());
                                break;
                            } else if !c.is_whitespace() {
                                break;
                            }
                        }
                    }
                }
            }
            if let Some(res) = res {
                *text = res.to_string();
            }
        }
    }

    for (n, item) in doc.0.iter_mut().enumerate() {
        if let Item::Text { text, .. } = item {
            *text = strip_indent(&text, &indent.s, n == 0);
        }
    }

    (doc, indent)
}

#[derive(Debug, Clone)]
struct Indent {
    open: bool,
    s: String,
}

impl Indent {
    fn new() -> Self {
        Indent {
            open: true,
            s: "".to_string(),
        }
    }
}

fn get_indent(s: &str) -> Indent {
    let mut indent = Indent::new();
    let mut indent_start = 0;
    let mut indent_end = 0;
    let mut in_indent = true;

    for (pos, c) in s.char_indices() {
        if c == '\n' {
            if in_indent {
                indent_end = pos;
            }
            let i = Indent {
                open: in_indent,
                s: (&s[indent_start..indent_end]).to_string(),
            };
            indent = unify_indents(indent, i);
            indent_start = pos + 1;
            in_indent = true;
        } else if in_indent {
            if !c.is_whitespace() {
                in_indent = false;
                indent_end = pos;
            }
        }
    }

    indent
}

fn unify_indents(s1: Indent, s2: Indent) -> Indent {
    let mut i1 = s1.s.char_indices();
    let mut i2 = s2.s.chars();

    loop {
        let c1 = i1.next();
        let c2 = i2.next();
        if let Some(c1) = c1 {
            if let Some(c2) = c2 {
                if c1.1 != c2 {
                    return Indent {
                        open: false,
                        s: (&s1.s[0..c1.0]).to_string(),
                    };
                }
            } else {
                return if s2.open { s1.clone() } else { s2.clone() }; // FIXME: don't clone
            }
        } else {
            return if s1.open { s2.clone() } else { s1.clone() }; // FIXME: don't clone
        }
    }
}

fn strip_indent(s: &str, indent: &str, strip_first: bool) -> String {
    let mut res = String::new();
    let mut i = s.chars().peekable();

    loop {
        /* Skip the indentation. */
        if strip_first {
            let mut j = indent.chars();
            loop {
                if let Some(c1) = i.peek() {
                    if let Some(c2) = j.next() {
                        if c1 != &c2 {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    return res;
                }
                i.next();
            }
        }

        /* Copy all characters up to and including the end-of-line. */
        loop {
            if let Some(c) = i.next() {
                res.push(c);
                if c == '\n' {
                    break;
                }
            } else {
                return res;
            }
        }
    }
}
