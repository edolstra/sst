use std::mem;
use std::cmp;

pub struct Block {
    margin_top: usize,
    margin_bottom: usize,
    content: Content,
}

impl Block {
    pub fn new(content: Content) -> Self {
        Block { margin_top: 1, margin_bottom: 1, content }
    }

    pub fn with_margin(margin_top: usize, margin_bottom: usize, content: Content) -> Self {
        Block { margin_top, margin_bottom, content }
    }
}

pub enum Content {
    Para(Texts),
    Pre(Texts),
    TB(Blocks),
    Table(Vec<Blocks>),
}

pub type Blocks = Vec<Block>;

#[derive(Debug, Clone)]
pub enum Text {
    Text(String),
    Styled(Style, Texts),
}

pub type Texts = Vec<Text>;

#[derive(Debug, Clone)]
pub enum Style {
    Bold,
    Italic,
    Underline,
    Strikethrough
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct FullStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

impl FullStyle {
    pub fn new() -> Self {
        FullStyle {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }

    pub fn apply(&self, style: &Style) -> Self {
        let mut res = self.clone();
        match style {
            Style::Bold => { res.bold = true; }
            Style::Italic => { res.italic = true; }
            Style::Underline => { res.underline = true; }
            Style::Strikethrough => { res.strikethrough = true; }
        };
        res
    }
}

type StyledLine = Vec<(FullStyle, char)>;
type StyledLines = Vec<StyledLine>;

fn layout(max_width: usize, mut margin_top_min: usize, block: &Block, lines: &mut StyledLines) {

    // FIXME: only emit margin when we emit some lines.
    if !lines.is_empty() {
        for n in 0..cmp::max(margin_top_min, block.margin_top) {
            lines.push(vec![]);
        }
    }

    match &block.content {

        Content::Para(text) => {

            let mut line = vec![];
            flatten_texts(text, &FullStyle::new(), &mut line);

            struct State<'a> {
                max_width: &'a usize,
                lines: &'a mut StyledLines,
                cur_line: StyledLine,
                cur_span: StyledLine,
                cur_whitespace: StyledLine,
                in_whitespace: bool,
            }

            impl<'a> State<'a> {
                fn push(&mut self, style: &FullStyle, c: char) {
                    if c.is_whitespace() {
                        if !self.in_whitespace {
                            // Flush the current word, if any.
                            self.flush_word();
                            self.in_whitespace = true;
                            self.cur_whitespace.push((style.clone(), ' '));
                        } else {
                            // Merge adjacent whitespace if it has the same style.
                            if !self.cur_whitespace.is_empty() && &self.cur_whitespace[0].0 != style {
                                self.cur_whitespace.push((style.clone(), ' '));
                            }
                        }
                    } else {
                        if self.in_whitespace {
                            self.in_whitespace = false;
                            self.cur_span = vec![];
                        }
                        self.cur_span.push((style.clone(), c));
                    }
                }

                fn flush_word(&mut self) {
                    if !self.cur_span.is_empty() {

                        if self.cur_line.len() + self.cur_span.len() >= *self.max_width {
                            self.flush_line();
                        }

                        if !self.cur_line.is_empty() {
                            self.cur_line.extend(mem::replace(&mut self.cur_whitespace, vec![]));
                        } else {
                            self.cur_whitespace = vec![];
                        }

                        self.cur_line.extend(mem::replace(&mut self.cur_span, vec![]));
                    }
                }

                fn flush_line(&mut self) {
                    if !self.cur_line.is_empty() {
                        self.lines.push(mem::replace(&mut self.cur_line, vec![]));
                    }
                }
            }

            let mut state = State {
                max_width: &max_width,
                lines: lines,
                cur_line: vec![],
                cur_span: vec![],
                cur_whitespace: vec![],
                in_whitespace: false
            };

            for (style, c) in &line {
                state.push(style, *c);
            }

            state.flush_word();
            state.flush_line();
        }

        Content::Pre(text) => {
            let mut line = vec![];
            flatten_texts(text, &FullStyle::new(), &mut line);
            for l in line.split(|c| c.1 == '\n') {
                lines.push(l.to_vec());
            }
        }

        Content::TB(blocks) => {
            for block in blocks {
                layout(max_width, margin_top_min, block, lines);
                margin_top_min = block.margin_bottom;
            }
        }

        Content::Table(rows) => {
            if rows.is_empty() { return; }

            let nr_columns = rows[0].len();

            let mut column_widths = vec![0; nr_columns];
            let mut row_heights = vec![0; rows.len()];
            let mut children = vec![];
            let mut width_left = max_width;

            for column_index in 0..nr_columns {
                /* Compute the width of this column. */
                let mut column_children = vec![];
                let mut column_width = 0;
                for (row_index, row) in rows.iter().enumerate() {
                    let child = &row[column_index];
                    let mut child_lines = vec![];
                    layout(width_left - if column_index + 1 == nr_columns {0} else {1},
                           0, &child, &mut child_lines);
                    for line in &child_lines {
                        column_width = cmp::max(column_width, line.len());
                    }
                    row_heights[row_index] = cmp::max(row_heights[row_index], child_lines.len());
                    column_children.push(child_lines);
                }

                children.push(column_children);

                width_left = if column_width < width_left { width_left - column_width - 1 } else { 1 };

                column_widths[column_index] = column_width;
            }

            for (row_index, row) in rows.iter().enumerate() {
                for line_nr in 0..row_heights[row_index] {
                    let mut line = vec![];
                    for (column_index, column) in row.iter().enumerate() {
                        let child = &children[column_index][row_index];
                        let l = if line_nr < child.len() {
                            children[column_index][row_index][line_nr].clone() // FIXME: move
                        } else { vec![] };
                        let l_width = l.len();
                        line.extend(l);
                        if column_index + 1 < nr_columns {
                            for n in 0..1 + column_widths[column_index] - l_width {
                                line.push((FullStyle::new(), ' '));
                            }
                        }
                    }

                    lines.push(line);

                    if row_index + 1 < rows.len() {
                        lines.push(vec![]);
                    }
                }
            }
        }

        _ => unimplemented!()
    }
}

fn flatten_texts(texts: &Texts, style: &FullStyle, line: &mut StyledLine) {
    for text in texts {
        match text {
            Text::Text(s) => {
                for c in s.chars() {
                    line.push((style.clone(), c));
                }
            },
            Text::Styled(change, texts2) => {
                flatten_texts(texts2, &style.apply(&change), line);
            }
        }
    }
}

fn emit_ansi_delta(dest: &mut String, old: &FullStyle, new: &FullStyle) {
    if old != new {
        dest.push_str("\x1b[0");
        if new.bold {
            dest.push_str(";1");
        }
        if new.italic {
            dest.push_str(";3");
        }
        if new.underline {
            dest.push_str(";4");
        }
        if new.strikethrough {
            dest.push_str(";9");
        }
        dest.push_str("m");
    }
}

fn apply_style(dest: &mut String, lines: &StyledLines) {
    let mut cur_style = FullStyle::new();
    for line in lines {
        for c in line {
            emit_ansi_delta(dest, &cur_style, &c.0);
            dest.push(c.1);
            cur_style = c.0.clone();
        }
        dest.push('\n');
        // Work around a bug in 'less -R': it resets the style at the
        // start of every line.
        cur_style = FullStyle::new();
    }
    emit_ansi_delta(dest, &cur_style, &FullStyle::new());
}

pub fn format(max_width: usize, block: &Block) -> String {
    let mut lines = vec![];
    layout(max_width, 0, &block, &mut lines);

    let mut res = String::new();
    apply_style(&mut res, &lines);
    res
}
