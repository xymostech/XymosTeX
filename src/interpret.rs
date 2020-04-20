#[macro_use]
extern crate lazy_static;

mod dimension;
mod dvi;
mod paths;
mod tfm;

use dvi::{interpret_dvi_file, DVIFile};
use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<String>>();

    if args.len() != 1 {
        panic!("Invalid number of arguments: {}", args.len());
    }

    let filename = args.pop().unwrap();
    let file = fs::File::open(filename)?;
    let dvi = DVIFile::new(file)?;

    let pages = interpret_dvi_file(dvi);
    for page in pages {
        for (k, v) in page.iter() {
            println!("{:?} {:?}", k, v);
        }
    }

    Ok(())
}
