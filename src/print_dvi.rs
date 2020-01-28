mod dvi;

use dvi::DVIFile;
use std::env;
use std::fs;

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<String>>();

    if args.len() != 1 {
        panic!("Invalid number of arguments: {}", args.len());
    }

    let filename = args.pop().unwrap();
    let file = fs::File::open(filename).unwrap();
    let dvi = DVIFile::new(file).unwrap();
    for command in dvi.commands {
        println!("{:?}", command);
    }
}
