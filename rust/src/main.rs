mod ast;
mod core;
mod eval;
mod number;
mod parser;
mod schema;
mod text_layout;
mod to_text;
mod unindent;
mod validate;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "sst", about = "Simple Structured Text processor")]
enum Sst {
    /// Parse an SST file
    #[structopt(name = "parse")]
    Parse {
        /// File to parse
        input: PathBuf,
    },

    /// Evaluate an SST file
    #[structopt(name = "eval")]
    Eval {
        /// File to evaluate
        input: PathBuf,
    },

    /// Validate an SST file
    #[structopt(name = "check")]
    Check {
        /// Print validation proof in JSON
        #[structopt(short = "j", long = "json")]
        json: bool,
        /// File to validate
        input: PathBuf,
    },

    /// Read an SST file in your terminal
    #[structopt(name = "read")]
    Read {
        /// File to read
        input: PathBuf,
    },
}

fn parse_file(filename: &Path, include_filename: bool) -> ast::Doc {
    let input = fs::read_to_string(&filename).expect("Unable to read file");
    parser::parse_string(
        if include_filename {
            Some(&filename)
        } else {
            None
        },
        &input,
    )
    .expect("Parse error")
}

fn eval_file(filename: &Path) -> ast::Doc {
    let ast = parse_file(filename, true);
    eval::eval(&ast).expect("Evaluation error")
}

fn validate_file(filename: &Path) -> validate::Instance {
    let ast = eval_file(filename);
    validate::validate(&core::SCHEMA, &ast, &filename).expect("Validation error")
}

fn show_in_pager(text: &str) {
    let is_tty = unsafe { libc::isatty(libc::STDOUT_FILENO as i32) } != 0;

    if is_tty {
        let mut process = Command::new("less")
            .arg("-R")
            .stdin(Stdio::piped())
            .spawn()
            .expect("Couldn't run pager");

        process
            .stdin
            .as_mut()
            .unwrap()
            .write_all(text.as_bytes())
            .expect("Couldn't write bytes to pager");

        process.wait().unwrap();
    } else {
        print!("{}", text);
    }
}

fn main() {
    match Sst::from_args() {
        Sst::Parse { input } => {
            let ast = parse_file(&input, false);
            println!("{}", &serde_json::to_string_pretty(&ast).unwrap());
        }

        Sst::Eval { input } => {
            let ast = eval_file(&input);
            println!("{}", &serde_json::to_string(&ast).unwrap());
        }

        Sst::Check { input, json } => {
            let instance = validate_file(&input);
            if json {
                println!("{}", &serde_json::to_string(&instance).unwrap());
            }
        }

        Sst::Read { input } => {
            let instance = validate_file(&input);
            let text = to_text::to_text(&instance, 80);
            show_in_pager(&text);
        }
    }
}
