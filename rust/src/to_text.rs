use crate::{number, text_layout::*, validate::*};

struct ToText<'doc> {
    numbers: number::Numbers<'doc>,
    max_width: usize,
}

pub fn to_text(doc: &Instance, max_width: usize) -> String {
    let state = ToText {
        numbers: number::Numbers::create(doc),
        max_width,
    };

    let mut blocks = vec![];
    state.toplevel(doc, &mut blocks);

    format(max_width, &Block::new(Content::TB(blocks)))
}

impl<'doc> ToText<'doc> {
    fn toplevel(&self, doc: &Instance, blocks: &mut Blocks) {
        let doc = doc.unchoice();
        match doc {
            Instance::Many(_) => {
                self.blocks(doc, blocks);
            }
            Instance::Element(tag, _) if tag == "book" => {
                self.book(doc, blocks);
            }
            Instance::Element(tag, _) if tag == "article" => {
                self.article(doc, blocks);
            }
            Instance::Element(tag, _) if tag == "part" => {
                self.part(doc, blocks);
            }
            Instance::Element(tag, _) if tag == "chapter" => {
                self.chapter(doc, blocks);
            }
            Instance::Element(tag, _) if tag == "section" => {
                self.section(doc, blocks);
            }
            _ => panic!(),
        }
    }

    fn book(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "book" => {
                let title = &children[0];
                let body = &children[1];
                let mut texts = vec![];
                self.inlines(title, &mut texts);
                blocks.push(Block::new(Content::Para(texts)));
                for item in body.iter() {
                    self.chapter(item, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn article(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "article" => {
                let title = &children[0];
                let body = &children[1].seq();
                let mut texts = vec![];
                self.inlines(title, &mut texts);
                blocks.push(Block::new(Content::Para(texts)));
                self.blocks(&body[0], blocks);
                for s in body[1].iter() {
                    self.simplesect(s, blocks);
                }
                for item in body[2].iter() {
                    self.section(item, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn part(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "part" => {
                let title = &children[0];
                let body = &children[1];
                let mut texts = vec![];
                self.inlines(title, &mut texts);
                blocks.push(Block::new(Content::Para(texts)));
                for item in body.iter() {
                    self.chapter(item, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn get_title(&self, doc: &Instance) -> String {
        self.numbers.get_toc_entry(doc).unwrap().to_string()
    }

    fn emit_title(&self, doc: &Instance, blocks: &mut Blocks) {
        let toc_entry = self
            .numbers
            .get_toc_entry(doc)
            .expect(&format!("Expected TOC entry for: {:?}", doc));
        let mut texts = vec![];
        texts.push(Text::Text(toc_entry.to_string()));
        texts.push(Text::Text(" ".to_string()));
        self.inlines(toc_entry.title, &mut texts);
        blocks.push(Block::new(Content::Para(vec![Text::Styled(
            Style::Bold,
            vec![Text::Styled(Style::Underline, texts)],
        )])));
    }

    fn chapter(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "chapter" => {
                let body = &children[1].seq();
                self.emit_title(&doc, blocks);
                self.blocks(&body[0], blocks);
                for s in body[1].iter() {
                    self.simplesect(s, blocks);
                }
                for item in body[2].iter() {
                    self.section(item, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn section(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "section" => {
                let body = &children[1].seq();
                self.emit_title(&doc, blocks);
                self.blocks(&body[0], blocks);
                for s in body[1].iter() {
                    self.simplesect(s, blocks);
                }
                for item in body[2].iter() {
                    self.subsection(item, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn subsection(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "subsection" => {
                let body = &children[1].seq();
                self.emit_title(&doc, blocks);
                self.blocks(&body[0], blocks);
                for s in body[1].iter() {
                    self.simplesect(s, blocks);
                }
            }
            _ => panic!(),
        }
    }

    fn simplesect(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc {
            Instance::Element(tag, children) if tag == "simplesect" => {
                let title = &children[0];
                let body = &children[1].seq();
                let mut texts = vec![];
                self.inlines(title, &mut texts);
                blocks.push(Block::new(Content::Para(vec![Text::Styled(
                    Style::Underline,
                    texts,
                )])));
                self.blocks(&body[0], blocks);
            }
            _ => panic!(),
        }
    }

    fn blocks(&self, doc: &Instance, blocks: &mut Blocks) {
        for item in doc.iter() {
            self.block(item, blocks);
        }
    }

    fn block(&self, doc: &Instance, blocks: &mut Blocks) {
        match doc.unchoice() {
            Instance::Para(para) => {
                let mut texts = vec![];
                self.inlines(para, &mut texts);
                blocks.push(Block::new(Content::Para(texts)));
            }
            Instance::Element(tag, children) if tag == "dinkus" => {
                //lines.push(Text::new("* * *".to_string()).center(max_width));
            }
            Instance::Element(tag, children) if tag == "listing" || tag == "screen" => {
                let mut texts = vec![];
                self.inlines(&children[0], &mut texts);
                blocks.push(Block::new(Content::Table(vec![vec![
                    Block::new(Content::Pre(vec![Text::Text("   ".to_string())])),
                    Block::new(Content::Pre(texts)),
                ]])));
            }
            Instance::Element(tag, children)
                if tag == "ol" || tag == "ul" || tag == "procedure" =>
            {
                let mut rows = vec![];
                for (n, step) in children[0].many().iter().enumerate() {
                    let mut blocks = vec![];
                    match step {
                        Instance::Element(tag, children) if tag == "li" || tag == "step" => {
                            self.blocks(&children[0], &mut blocks);
                        }
                        _ => unreachable!(),
                    }
                    rows.push(vec![
                        Block::new(Content::Pre(vec![Text::Text(if tag == "ul" {
                            "*".to_string()
                        } else {
                            format!("{}.", n + 1)
                        })])),
                        Block::new(Content::TB(blocks)),
                    ]);
                }
                blocks.push(Block::new(Content::Table(rows)));
            }
            Instance::Element(tag, children) if tag == "namedlist" => {
                for step in children[0].many().iter() {
                    match step {
                        Instance::Element(tag, children) if tag == "item" => {
                            let mut texts = vec![];
                            texts.push(Text::Text("* ".to_string()));
                            self.inlines(&children[0], &mut texts);
                            blocks.push(Block::new(Content::Para(vec![Text::Styled(
                                Style::Bold,
                                texts,
                            )])));

                            let mut blocks2 = vec![];
                            self.blocks(&children[1], &mut blocks2);
                            blocks.push(Block::new(Content::Table(vec![vec![
                                Block::new(Content::Pre(vec![Text::Text(" ".to_string())])),
                                Block::new(Content::TB(blocks2)),
                            ]])));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => panic!("Unsupported: {:?}", doc.unchoice()),
        }
    }

    fn inlines(&self, doc: &Instance, texts: &mut Texts) {
        if let Instance::Many(docs) = doc {
            for d in docs {
                match d.unchoice() {
                    Instance::Text(s) => {
                        texts.push(Text::Text(s.clone()));
                    }
                    Instance::Element(tag, children) if tag == "emph" => {
                        let mut texts2 = vec![];
                        self.inlines(&children[0], &mut texts2);
                        texts.push(Text::Styled(Style::Italic, texts2));
                    }
                    Instance::Element(tag, children) if tag == "strong" => {
                        let mut texts2 = vec![];
                        self.inlines(&children[0], &mut texts2);
                        texts.push(Text::Styled(Style::Bold, texts2));
                    }
                    Instance::Element(tag, children) if tag == "todo" => {
                        let mut texts2 = vec![];
                        texts2.push(Text::Text("[".to_string()));
                        self.inlines(&children[0], &mut texts2);
                        texts2.push(Text::Text("]".to_string()));
                        texts.push(Text::Styled(
                            Style::Bold,
                            vec![Text::Styled(Style::Color(Color::Red), texts2)],
                        ));
                    }
                    Instance::Element(tag, children) if tag == "code" || tag == "filename" => {
                        let mut texts2 = vec![];
                        self.inlines(&children[0], &mut texts2);
                        texts.push(Text::Styled(Style::Bold, texts2));
                    }
                    Instance::Element(tag, children) if tag == "link" => {
                        self.inlines(&children[1], texts);
                        if let Instance::Text(s) = &children[0] {
                            texts.push(Text::Text(" (".to_string()));
                            texts.push(Text::Styled(Style::Bold, vec![Text::Text(s.clone())]));
                            texts.push(Text::Text(")".to_string()));
                        }
                    }
                    _ => {
                        texts.push(Text::Text("<UNHANDLED>".to_string()));
                    }
                }
            }
        } else {
            panic!("{:?}", doc)
        }
    }
}
