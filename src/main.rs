#[macro_use]
extern crate lazy_static;

use std::{
    env, fs,
    io::{self, Read},
};
pub mod plainsong;

fn help() {
    println!("Usage: plainsong <to-ron|to-latex> [filename]");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut content = String::new();
    match args.len() {
        2 => {
            eprintln!("Filename has been omitted, reading from stdin");
            io::stdin().read_to_string(&mut content).unwrap();
        }
        3 => {
            eprintln!("Reading plain song from {}", args[2]);
            content = fs::read_to_string(&args[2]).expect("Something went wrong reading the file");
        }
        _ => {
            help();
            return;
        }
    }

    let mut song = plainsong::SongParser::parse(&content);

    match args[1].as_ref() {
        "to-ron" => {
            println!("{:#?}", song);
        }
        "to-latex" => {
            println!("{}", &song.to_latex());
        }
        _ => {
            help();
            return;
        }
    }
}
