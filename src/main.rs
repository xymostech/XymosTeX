#![deny(clippy::all)]

mod box_to_dvi;
mod boxes;
mod category;
mod dimension;
mod dvi;
mod font;
mod font_metrics;
mod glue;
mod lexer;
mod list;
mod makro;
mod math_code;
mod math_list;
mod parser;
mod paths;
mod state;
mod tfm;
mod token;
mod variable;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use std::fs;
use std::io;
use std::io::prelude::*;

use crate::box_to_dvi::DVIFileWriter;
use crate::parser::Parser;
use crate::state::TeXState;

fn main() -> io::Result<()> {
    let mut lines: Vec<String> = Vec::new();

    // Read in every line of stdin. This currently doesn't let us do parsing as
    // we go along, but that's fine.
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        lines.push(line?);
    }

    let state = TeXState::new();
    let mut parser = Parser::new(&lines[..], &state);

    let mut file_writer = DVIFileWriter::new();
    file_writer.start(
        (25400000, 473628672),
        1000,
        b"Made by XymosTeX".to_vec(),
    );

    let result = parser.parse_outer_vertical_box();
    file_writer.add_page(&result.list, &None, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    file_writer.end();

    let file = file_writer.to_file();

    let output = fs::File::create("texput.dvi")?;
    file.write_to(output)
}
