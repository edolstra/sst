mod ast;
mod core;
mod eval;
mod number;
mod parser;
mod schema;
mod text_layout;
mod to_text;
mod validate;

use clap::{App, Arg, SubCommand};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn parse_file(filename: &str) -> ast::Doc {
    let input = fs::read_to_string(&filename).expect("Unable to read file");
    parser::parse_string(&filename, &input).expect("Parse error")
}

fn eval_file(filename: &str) -> ast::Doc {
    let ast = parse_file(filename);
    eval::eval(&ast).expect("Evaluation error")
}

fn validate_file(filename: &str) -> validate::Instance {
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
    let app = App::new("SST processor")
        .version("0.1")
        .author("Eelco Dolstra <edolstra@gmail.com>")
        .subcommand(
            SubCommand::with_name("parse")
                .about("Parse an SST file")
                .arg(Arg::with_name("INPUT").help("File to parse").required(true)),
        )
        .subcommand(
            SubCommand::with_name("eval")
                .about("Evaluate an SST file")
                .arg(
                    Arg::with_name("INPUT")
                        .help("File to evaluate")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("Validate an SST file")
                .arg(
                    Arg::with_name("INPUT")
                        .help("File to validate")
                        .required(true),
                )
                .arg(
                    Arg::with_name("json")
                        .long("json")
                        .short("j")
                        .help("Print validation proof in JSON"),
                ),
        )
        .subcommand(
            SubCommand::with_name("read")
                .about("Read an SST file in your terminal")
                .arg(Arg::with_name("INPUT").help("File to read").required(true)),
        );

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("parse") {
        let filename = matches.value_of("INPUT").unwrap();
        let ast = parse_file(&filename);
        println!("{}", &serde_json::to_string(&ast).unwrap());;
    } else if let Some(matches) = matches.subcommand_matches("eval") {
        let filename = matches.value_of("INPUT").unwrap();
        let ast = eval_file(&filename);
        println!("{}", &serde_json::to_string(&ast).unwrap());;
    } else if let Some(matches) = matches.subcommand_matches("check") {
        let filename = matches.value_of("INPUT").unwrap();
        let instance = validate_file(&filename);
        if matches.is_present("json") {
            println!("{}", &serde_json::to_string(&instance).unwrap());
        }
    } else if let Some(matches) = matches.subcommand_matches("read") {
        let filename = matches.value_of("INPUT").unwrap();
        let instance = validate_file(&filename);
        //text_layout::layout_test();
        let text = to_text::to_text(&instance, 80);
        show_in_pager(&text);
    } else {
        eprintln!("{}", matches.usage());
    }
}
