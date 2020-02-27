mod dvi;

use dvi::DVIFile;
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
    for command in dvi.commands {
        println!("{:?}", command);
    }
    Ok(())
}
