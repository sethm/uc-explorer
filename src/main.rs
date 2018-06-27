///
/// Copyright 2017, Seth J. Morabito <web@loomcom.com>
///
/// This file is part of the Symbolics Microcode Explorer.
///
/// The Symbolics Microde Explorer is free software: you can
/// redistribute it and/or modify it under the terms of the GNU
/// General Public License as published by the Free Software
/// Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// The Symbolics Microde Explorer is distributed in the hope that it
/// will be useful, but WITHOUT ANY WARRANTY; without even the implied
/// warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
/// See the GNU General Public License for more details.
///
/// You should have received a copy of the GNU General Public License
/// along with The Symbolics Microde Explorer.  If not, see
/// <https://www.gnu.org/licenses/>.
///

extern crate clap;
use clap::{Arg, App};
use std::error::Error;
use std::fs::File;
use std::path::Path;

pub mod ucode;

fn main() {
    let app = App::new("Symbolics Microcode Explorer")
        .version("1.0")
        .author("Seth Morabito <web@loomcom.com>")
        .about("Parses and displays details of Symbolics 3600 microcode")
        .arg(
            Arg::with_name("INPUT")
                .help("Input file")
                .required(true)
                .index(1),
        )
        .get_matches();

    let infile = app.value_of("INPUT").unwrap();

    let path = Path::new(infile);
    let display_name = path.display();

    let file = match File::open(&path) {
        Ok(file) => file,
        Err(why) => {
            eprintln!(
                "Couldn't open input file {}: {}",
                display_name,
                why.description()
            );
            std::process::exit(1);
        }
    };

    let mut state = ucode::Microcode::new(&file);

    match state.load() {
        Ok(()) => {
            println!("--------------------= STATE =------------------------");
            println!("{}", state);
            println!("-----------------------------------------------------");
        },
        Err(reason) => println!("Unable to parse microcode: {}", reason),
    }
}
