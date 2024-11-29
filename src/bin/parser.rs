use std::{env, fs};

use sgleam::parser::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    let src = fs::read_to_string(&args[1]).unwrap();

    for item in parse_repl(&src).unwrap() {
        println!("{item:#?}");
    }
}
