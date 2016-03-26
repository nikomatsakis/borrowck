extern crate docopt;
extern crate nll_repr;
extern crate rustc_serialize;

use docopt::Docopt;
use nll_repr::repr::*;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

fn main() {
    let args: Args =
        Docopt::new(USAGE)
        .and_then(|d| d.argv(env::args()).decode())
        .unwrap_or_else(|e| e.exit());

    for input in &args.arg_inputs {
        match process_input(input) {
            Ok(()) => { }
            Err(err) => {
                println!("Error with {}: {}",
                         input, err);
            }
        }
    }
}

fn process_input(input: &str) -> Result<(), Box<Error>> {
    let ballast = Ballast::new();
    let mut arena = Arena::new(&ballast);
    let mut file_text = String::new();
    let mut file = try!(File::open(input));
    if file.read_to_string(&mut file_text).is_err() {
        return try!(Err(String::from("not UTF-8")));
    }
    let func = try!(Func::parse(&mut arena, &file_text));
    println!("{:?}", func);
    Ok(())
}

const USAGE: &'static str = "
Usage: nll <inputs>...
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_inputs: Vec<String>,
}
