#![deny(clippy::all)]
#![allow(clippy::new_without_default)]
#![cfg_attr(test, allow(clippy::disallowed_types))]

pub mod box_to_dvi;
mod boxes;
mod category;
mod dimension;
pub mod dvi;
mod font;
mod font_metrics;
mod glue;
mod lexer;
mod line_breaking;
mod list;
mod makro;
mod math_code;
mod math_list;
pub mod parser;
mod paths;
pub mod state;
mod tfm;
mod token;
mod variable;

#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;
