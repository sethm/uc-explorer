///
/// Copyright 2017, Seth J. Morabito <web@loomcom.com>
///
/// This file is part of the Symbolics Microcode Explorer.
///
/// The Symbolics Microcode Explorer is free software: you can
/// redistribute it and/or modify it under the terms of the GNU
/// General Public License as published by the Free Software
/// Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// The Symbolics Microcode Explorer is distributed in the hope that it
/// will be useful, but WITHOUT ANY WARRANTY; without even the implied
/// warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
/// See the GNU General Public License for more details.
///
/// You should have received a copy of the GNU General Public License
/// along with The Symbolics Microde Explorer.  If not, see
/// <https://www.gnu.org/licenses/>.
///
extern crate clap;
extern crate rustyline;

pub mod ucode;

use clap::{App, Arg};

use rustyline::completion::FilenameCompleter;
use rustyline::hint::Hinter;
use rustyline::{CompletionType, Config, EditMode, Editor};

use std::fmt;
use std::fs::File;
use std::io;
use std::io::Write;
use std::vec::Vec;
use ucode::Microcode;

struct Hints {}

impl Hinter for Hints {
    fn hint(&self, _line: &str, _pos: usize) -> Option<String> {
        None // Hints are not yet implemented
    }
}

pub enum HandlerError {
    Io(io::Error),
    ParseError,
}

pub enum HandlerResult {
    Handled,
    Quit,
}

impl From<io::Error> for HandlerError {
    fn from(err: io::Error) -> HandlerError {
        HandlerError::Io(err)
    }
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HandlerError::Io(ref err) => err.fmt(f),
            HandlerError::ParseError => write!(f, "Parse Error"),
        }
    }
}

fn do_dump(ucode: &mut Microcode, words: Vec<&str>) -> Result<HandlerResult, HandlerError> {
    if words.len() == 2 {
        println!("Dumping to file {}...", words[1]);

        let mut file = File::create(&words[1])?;

        // TODO: There must be a better way to write this.
        match write!(file, "{}", ucode) {
            Ok(_) => {}
            Err(e) => return Err(HandlerError::Io(e)),
        }
    } else {
        println!("usage: dump [filename]");
    }

    Ok(HandlerResult::Handled)
}

fn do_show(ucode: &mut Microcode, _words: Vec<&str>) -> Result<HandlerResult, HandlerError> {
    if ucode.path.is_some() {
        println!("Loaded From:     {}", ucode.path());
        println!("Version:         {}", ucode.version);
        println!("Commend:         {}", ucode.comment);
        println!("A-Mem Size:      {} words", ucode.a_mem.len());
        println!("B-Mem Size:      {} words", ucode.b_mem.len());
        println!("C-Mem Size:      {} words", ucode.c_mem.len());
        println!("Type Map Size:   {} words", ucode.type_map.len());
        println!("Pico Store Size: {} words", ucode.pico_store.len());
    } else {
        println!("No microcode is loaded.");
    }

    Ok(HandlerResult::Handled)
}

fn do_load(ucode: &mut Microcode, words: Vec<&str>) -> Result<HandlerResult, HandlerError> {
    if words.len() == 2 {
        match ucode.load(words[1]) {
            Ok(()) => {
                println!("Loaded file {}", ucode.path());
            }
            Err(e) => {
                println!("Cannot load file. {}", e);
            }
        }
    } else {
        println!("usage: load [filename]");
    }

    Ok(HandlerResult::Handled)
}

// TODO: Automatically generate help from command list.
fn do_help() -> Result<HandlerResult, HandlerError> {
    println!("help                Show this help.");
    println!("load [file]         Load a Microcode file.");
    println!("dump [file]         Disassemble to file.");
    println!("show                Show microcode overview.");
    println!("q,quit              Leave the shell.");

    Ok(HandlerResult::Handled)
}

fn handle_command(ucode: &mut Microcode, input: &str) -> Result<HandlerResult, HandlerError> {
    let words = input.split(" ").collect::<Vec<&str>>();

    if words.len() == 0 {
        Err(HandlerError::ParseError)
    } else {
        match words[0] {
            "quit" => Ok(HandlerResult::Quit),
            "q" => Ok(HandlerResult::Quit),
            "help" => do_help(),
            "dump" => do_dump(ucode, words),
            "show" => do_show(ucode, words),
            "load" => do_load(ucode, words),
            "" => Ok(HandlerResult::Handled),
            _ => Err(HandlerError::ParseError),
        }
    }
}

/// Main processing loop
fn process_loop(ucode: &mut Microcode) {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let completer = FilenameCompleter::new();

    let mut rl = Editor::with_config(config);
    rl.set_helper(Some((completer, Hints {})));

    loop {
        let readline = rl.readline("uc-explorer> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());
                match handle_command(ucode, line.as_ref()) {
                    Ok(HandlerResult::Handled) => {
                        // Normal result. Continue looping.
                    }
                    Ok(HandlerResult::Quit) => {
                        // Quit detected. Break from loop.
                        break;
                    }
                    Err(HandlerError::Io(e)) => {
                        // IO error. Display failure, keep looping.
                        println!("Command failed. {}", e);
                    }
                    Err(HandlerError::ParseError) => {
                        // Parse error. Display failure, keep looping.
                        println!("?");
                    }
                }
            }
            Err(e) => {
                println!("Unrecoverable error: {}", e);
                break;
            }
        }
    }
}

fn main() {
    let app = App::new("Symbolics Microcode Explorer")
        .version("1.0")
        .author("Seth Morabito <web@loomcom.com>")
        .about("Parses and displays details of Symbolics 3600 microcode")
        .arg(
            Arg::with_name("file")
                .short("f")
                .help("Input file")
                .takes_value(true),
        )
        .get_matches();

    let mut state = Microcode::new();

    let file = app.value_of("file");

    match file {
        Some(f) => match state.load(f) {
            Ok(()) => process_loop(&mut state),
            Err(reason) => println!("Unable to parse microcode: {}", reason),
        },
        None => {
            process_loop(&mut state);
        }
    }
}
