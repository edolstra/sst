#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod ast;
mod parser;
mod eval;
mod schema;
mod validate;
mod core;

use std::fs;
use std::env;

fn main() {
    let filename = env::args().collect::<Vec<_>>()[1].clone();
    let input = fs::read_to_string(&filename).expect("Unable to read file");
    let ast = parser::parse_string(&filename, &input).expect("Parse error");
    let ast = eval::eval(&ast).expect("Evaluation error");
    //println!("{:#?}", ast);
    let instance = validate::validate(&core::SCHEMA, &ast, &filename).expect("Validation error");
    println!("{}", &serde_json::to_string(&instance).unwrap());
}
