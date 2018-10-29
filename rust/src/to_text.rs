use validate::*;

pub fn to_text(doc: &Instance, max_width: usize) -> String {
    let mut lines = vec![];

    toplevel(doc, max_width, &mut lines);

    let mut res = "".to_string();
    let mut need_space = false;
    for line in lines {
        if line.space_before && need_space {
            res.push('\n');
        }
        res.push_str(&line.text);
        res.push('\n');
        need_space = true;
    }
    res
}

type Lines = Vec<Text>;
type Words = Vec<Text>;

#[derive(Debug, Clone)]
struct Text {
    text: String,
    width: usize,
    space_before: bool,
    space_after: bool,
}

impl Text {
    fn new(text: String) -> Self {
        Self { width: text.len(), text, space_before: true, space_after: true }
    }

    fn ansi(text: String) -> Self {
        Self { width: 0, text, space_before: false, space_after: false }
    }

    fn unpadded(text: String) -> Self {
        Self { width: text.len(), text, space_before: false, space_after: false }
    }

    fn push(&mut self, text: &Text) {
        self.text.push_str(&text.text);
        self.width += text.width;
    }

    fn is_empty(&self) -> bool { self.text.is_empty() }

    fn center(mut self, max_width: usize) -> Self {
        if self.width < max_width {
            self.text = " ".repeat((max_width - self.width) / 2) + &self.text;
        }
        self
    }
}

fn toplevel(doc: &Instance, max_width: usize, lines: &mut Lines) {
    match doc.unchoice() {
        Instance::Element(tag, children) if tag == "book" => {
            let title = &children[0];
            let body = &children[1];
            let mut words = vec![];
            inlines(title, &mut words);
            words_to_lines(&words, max_width, lines);
            for item in body.iter() {
                chapter(item, max_width, lines);
            }
        }
        Instance::Element(tag, children) if tag == "chapter" => {
            chapter(doc, max_width, lines);
        }
        _ => panic!(),
    }
}

fn chapter(doc: &Instance, max_width: usize, lines: &mut Lines) {
    match doc.unchoice() {
        Instance::Element(tag, children) if tag == "chapter" => {
            let title = &children[0];
            let body = &children[1].seq();
            let mut words = vec![];
            inlines(title, &mut words);
            words_to_lines(&words, max_width, lines);
            blocks(&body[0], max_width, lines);
            for s in body[1].iter() {
                simplesect(s, max_width, lines);
            }
            //let sections = &body[2];
        }
        _ => panic!(),
    }
}

fn simplesect(doc: &Instance, max_width: usize, lines: &mut Lines) {
    match doc.unchoice() {
        Instance::Element(tag, children) if tag == "simplesect" => {
            let title = &children[0];
            let body = &children[1].seq();
            let mut words = vec![];
            inlines(title, &mut words);
            words_to_lines(&words, max_width, lines);
            blocks(&body[0], max_width, lines);
        }
        _ => panic!(),
    }
}

fn blocks(doc: &Instance, max_width: usize, lines: &mut Lines) {
    for item in doc.iter() {
        block(item, max_width, lines);
    }
}

fn block(doc: &Instance, max_width: usize, lines: &mut Lines) {
    match doc.unchoice() {
        Instance::Para(para) => {
            let mut words = vec![];
            inlines(para, &mut words);
            words_to_lines(&words, max_width, lines);
        }
        Instance::Element(tag, children) if tag == "dinkus" => {
            lines.push(Text::new("* * *".to_string()).center(max_width));
        }
        Instance::Element(tag, children) if tag == "listing" => {
        }
        _ => panic!("Unsupported: {:?}", doc)
    }
}

fn inlines(doc: &Instance, words: &mut Words) {
    if let Instance::Many(docs) = doc {
        for d in docs {
            match d.unchoice() {
                Instance::Text(s) => {
                    split_into_words(s, words);
                }
                Instance::Element(tag, children) if tag == "emph" => {
                    //words.push(Text::unpadded("*".to_string()));
                    words.push(Text::ansi("\x1b[3m".to_string()));
                    inlines(&children[0], words);
                    words.push(Text::ansi("\x1b[0m".to_string()));
                    //words.push(Text::unpadded("*".to_string()));
                }
                Instance::Element(tag, children) if tag == "remark" => {
                    words.push(Text::unpadded("[".to_string()));
                    inlines(&children[0], words);
                    words.push(Text::unpadded("]".to_string()));
                }
                Instance::Element(tag, children) if tag == "filename" => {
                    words.push(Text::ansi("\x1b[4m".to_string()));
                    inlines(&children[0], words);
                    words.push(Text::ansi("\x1b[0m".to_string()));
                }
                Instance::Element(tag, children) if tag == "code" => {
                    words.push(Text::ansi("\x1b[4m".to_string()));
                    inlines(&children[0], words);
                    words.push(Text::ansi("\x1b[0m".to_string()));
                }
                _ => panic!()
            }
        }
    } else { panic!() }
}

fn split_into_words(s: &str, words: &mut Words) {
    let mut space_before = false;
    let mut word = String::new();

    for c in s.chars() {
        if c.is_whitespace() {
            if !word.is_empty() {
                words.push(Text { width: word.len(), text: word, space_before, space_after: true });
                word = String::new();
            }
            space_before = true;
        } else {
            word.push(c);
        }
    }

    if !word.is_empty() {
        words.push(Text { width: word.len(), text: word, space_before, space_after: false });
    }
}

fn words_to_lines(words: &Words, max_width: usize, lines: &mut Lines) {
    let space = Text::unpadded(" ".to_string());
    let mut cur_word = Text::unpadded("".to_string());
    let mut cur_line = Text::unpadded("".to_string());
    cur_line.space_before = true;

    // Note: we add a dummy element to the iterator to force the last
    // word to be flushed.
    for word in words.iter().chain([Text::new("".to_string())].iter()) {
        if !cur_word.space_after && !word.space_before {
            cur_word.push(&word);
            cur_word.space_after = word.space_after;
        } else {
            if !cur_word.is_empty() {
                if cur_line.is_empty() {
                    cur_line.push(&cur_word);
                    cur_word = Text::unpadded("".to_string());
                } else {
                    if cur_line.width + 1 + cur_word.width <= max_width {
                        cur_line.push(&space);
                        cur_line.push(&cur_word);
                    } else {
                        lines.push(cur_line);
                        cur_line = Text::unpadded("".to_string());
                        cur_line.push(&cur_word);
                    }
                }
            }

            cur_word = word.clone();
        }
    }

    lines.push(cur_line);
}
