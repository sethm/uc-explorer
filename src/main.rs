///
/// Copyright 2017, Seth J. Morabito <web@loomcom.com>
///
/// This file is part of the Symbolics Microcode Explorer.
/// 
/// Foobar is free software: you can redistribute it and/or modify
/// it under the terms of the GNU General Public License as published by
/// the Free Software Foundation, either version 3 of the License, or
/// (at your option) any later version.
///
/// Foobar is distributed in the hope that it will be useful,
/// but WITHOUT ANY WARRANTY; without even the implied warranty of
/// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
/// GNU General Public License for more details.
///
/// You should have received a copy of the GNU General Public License
/// along with Foobar.  If not, see <https://www.gnu.org/licenses/>./
///

extern crate clap;
use clap::{Arg, App};
use std::io::Read;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::fmt;

//
// Error handling
//

enum MicrocodeError {
    Io(std::io::Error),
    InvalidHeader,
    InvalidVersion,
    InvalidComment,
    InvalidABMem,
    InvalidCMem,
    InvalidTypeMap,
    InvalidPicoStore,
    InvalidPicoStoreEof,
}

impl From<std::io::Error> for MicrocodeError {
    fn from(err: std::io::Error) -> MicrocodeError {
        MicrocodeError::Io(err)
    }
}

impl fmt::Display for MicrocodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MicrocodeError::Io(ref err) => err.fmt(f),
            MicrocodeError::InvalidHeader => write!(f, "Invalid header"),
            MicrocodeError::InvalidVersion => write!(f, "Invalid version"),
            MicrocodeError::InvalidComment => write!(f, "Invalid comment"),
            MicrocodeError::InvalidABMem => write!(f, "Invalid A/B memory"),
            MicrocodeError::InvalidCMem => write!(f, "Invalid C memory"),
            MicrocodeError::InvalidTypeMap => write!(f, "Invalid Type Map"),
            MicrocodeError::InvalidPicoStore => write!(f, "Invalid Pico-store"),
            MicrocodeError::InvalidPicoStoreEof => write!(f, "Invalid Pico-store EOF"),
        }
    }
}

//
// Microcode State
//

macro_rules! read_u8 {
    ($file:expr) => {
        {
            let mut buf = [0;1];
            $file.read_exact(&mut buf)?;
            buf[0]
        }
    }
}

macro_rules! read_u16 {
    ($file:expr) => {
        {
            let mut buf = [0;2];
            $file.read_exact(&mut buf)?;
            buf[0] as u16 | (buf[1] as u16) << 8
        }
    }
}

macro_rules! read_u32 {
    ($file:expr) => {
        {
            let mut buf = [0;4];
            $file.read_exact(&mut buf)?;
            buf[0] as u32 | (buf[1] as u32) << 8 |
            (buf[2] as u32) << 16 | (buf[3] as u32) << 24
        }
    }
}

macro_rules! read_abword {
    ($address:expr, $file:expr) => {
        {
            let mut buf = [0; 5];
            $file.read_exact(&mut buf)?;
            ABWord {
                address: $address,
                data: (buf[0] as u64 | (buf[1] as u64) << 8 |
                       (buf[2] as u64) << 16 | (buf[3] as u64) << 24 |
                       (buf[4] as u64) << 32)

            }
        }
    }
}

macro_rules! read_cword {
    ($address:expr, $file:expr) => {
        {
            let mut buf = [0; 14];
            $file.read_exact(&mut buf)?;
            CWord::new($address,
                       // Low 64 bits
                       buf[0] as u64 | (buf[1] as u64) << 8 |
                       (buf[2] as u64) << 16 | (buf[3] as u64) << 24 |
                       (buf[4] as u64) << 32 | (buf[5] as u64) << 40 |
                       (buf[6] as u64) << 48 | (buf[7] as u64) << 56,
                       // High 64 bits
                       buf[8] as u64 | (buf[9] as u64) << 8 |
                       (buf[10] as u64) << 16 | (buf[11] as u64) << 24 |
                       (buf[12] as u64) << 32 | (buf[13] as u64) << 40)
        }
    }
}

macro_rules! read_pico_store_word {
    ($file:expr) => {
        {
            PicoStoreWord {
                address: read_u16!($file),
                data: read_u32!($file),
            }
        }
    }
}

const HEADER_MAGIC: u8 = 5;

const SEC_HEADER: u8 = 1;
const SEC_VERSION: u8 = 2;
const SEC_COMMENT: u8 = 3;
const SEC_AMEM: u8 = 4;
const SEC_BMEM: u8 = 5;
const SEC_CMEM: u8 = 6;
const SEC_TYPEMAP: u8 = 7;
const SEC_EOF: u8 = 8;
const SEC_PICOSTORE: u8 = 10;

struct ABWord {
    address: u16,
    data: u64,
}

struct CWord {
    address: u16,
    data_l: u64,
    data_h: u64,
}

impl CWord {
    fn new(address: u16, data_l: u64, data_h: u64) -> CWord {
        CWord {
            address: address,
            data_l: data_l,
            data_h: data_h,
        }
    }
}

/// All the fields of a Microinstruction
#[allow(dead_code)]
struct MicroInstruction {
    u_amra: u16,           // bits 11-0:    A Mem Read Address (0-7777)
    u_r_base: u8,          // bits 10-9:    A Mem R Base Register Select (0-3)
    u_amra_sel: u8,        // bits 13-12:   A Mem Read Address interpretation (0-3)
    u_xybus_sel: u8,       // bit  14:      X & Y Bus Select (0-1)
    u_stkp_count: u8,      // bit  15:      Stack Pointer Count true/false (0-1)
    u_amwa: u16,           // bits 27-16:   A Mem Write Address (0-7777)
    lbus_dev: u16,         // bits 25-16:   LBUS dev (0-1777)
    u_w_base: u8,          // bits 26-25:   A Mem W Base Register Select (0-3)
    u_stkp_count_dir: u8,  // bit  27:      Same as bit 26 (0-1)
    u_amwa_sel: u8,        // bits 29-28:   A Mem Write Address interpretation (0-3)
    u_seq: u8,             // bits 31-30:   Sequencer Function (0-3)
    u_bmra: u8,            // bits 39-32:   B Memory Read Address (0-377)
    u_bmwa: u8,            // bits 43-40:   B Memory Write Address (0-17)
    u_bmem_from_xbus: u8,  // bit  44:      B Memory Write Data Select (0-1)
    u_mem: u8,             // bits 47-45:   Memory Control Function (0-7)
    u_spec: u8,            // bits 52-48:   Special Function (0-37)
    u_magic: u8,           // bits 56-53:   Magic Number (0-17)
    u_cond_sel: u8,        // bits 61-57:   Condition Select (0-37)
    u_cond_func: u8,       // bits 63-62:   Condition Function (0-3)
    u_alu: u8,             // bits 67-64:   ALU Function (0-17)
    u_byte_f: u8,          // bits 69-68:   Byte Function (0-3)
    u_obus_cdr: u8,        // bits 72-70:   Obus CDR code select (0-7)
    u_obus_htype: u8,      // bits 75-73:   Obus high type field select (0-7)
    u_obus_ltype_sel: u8,  // bit  76:      Obus low type field select (0-1)
    u_cpc_sel: u8,         // bits 78-77:   Next microprogram address select (0-3)
    u_npc_sel: u8,         // bit  79:      Next next micro addres select (0-1)
    u_naf: u16,            // bits 93-80:   Next Address Field (0-37777)
    u_speed: u8,           // bits 95-94:   Clock speed control (0-3)
    u_type_map_sel: u8,    // bits 101-96:  Type map select (0-77)
    u_au_op: u8,           // bits 109-102: FPA control (0-377)
    u_spare: u8,           // bit  110:     Spare bit
    u_parity_bit: u8,      // bit  111:     Parity bit
}

impl MicroInstruction {
    fn new(cword: &CWord) -> MicroInstruction {
        MicroInstruction {
            // Fields from the low 64-bits
            u_amra: (cword.data_l & 0xfff) as u16,
            u_r_base: ((cword.data_l >> 9) & 0x3) as u8,
            u_amra_sel: ((cword.data_l >> 12) & 0x3) as u8,
            u_xybus_sel: ((cword.data_l >> 14) & 0x1) as u8,
            u_stkp_count: ((cword.data_l >> 15) & 0x1) as u8,
            u_amwa: ((cword.data_l >> 16) & 0xfff) as u16,
            lbus_dev: ((cword.data_l >> 16) & 0x3ff) as u16,
            u_w_base: ((cword.data_l >> 25) & 0x3) as u8,
            u_stkp_count_dir: ((cword.data_l >> 27) & 0x1) as u8,
            u_amwa_sel: ((cword.data_l >> 28) & 0x3) as u8,
            u_seq: ((cword.data_l >> 30) & 0x3) as u8,
            u_bmra: ((cword.data_l >> 32) & 0xff) as u8,
            u_bmwa: ((cword.data_l >> 40) & 0xf) as u8,
            u_bmem_from_xbus: ((cword.data_l >> 44) & 0x1) as u8,
            u_mem: ((cword.data_l >> 45) & 0x7) as u8,
            u_spec: ((cword.data_l >> 48) & 0x1f) as u8,
            u_magic: ((cword.data_l >> 53) & 0xf) as u8,
            u_cond_sel: ((cword.data_l >> 57) & 0x1f) as u8,
            u_cond_func: ((cword.data_l >> 62) & 0x3) as u8,

            // Fields from the high 64-bits
            u_alu: (cword.data_h & 0xf) as u8,
            u_byte_f: ((cword.data_h >> 4) & 0x3) as u8,
            u_obus_cdr: ((cword.data_h >> 6) & 0x7) as u8,
            u_obus_htype: ((cword.data_h >> 9) & 0x7) as u8,
            u_obus_ltype_sel: ((cword.data_h >> 12) & 0x1) as u8,
            u_cpc_sel: ((cword.data_h >> 13) & 0x3) as u8,
            u_npc_sel: ((cword.data_h >> 15) & 0x1) as u8,
            u_naf: ((cword.data_h >> 16) & 0x3fff) as u16,
            u_speed: ((cword.data_h >> 30) & 0x3) as u8,
            u_type_map_sel: ((cword.data_h >> 32) & 0x3f) as u8,
            u_au_op: ((cword.data_h >> 38) & 0xff) as u8,
            u_spare: ((cword.data_h >> 46) & 0x1) as u8,
            u_parity_bit: ((cword.data_h >> 47) & 0x1) as u8,
        }
    }
}

#[allow(dead_code)]
struct TypeWord {
    data: u8,
}

#[allow(dead_code)]
struct PicoStoreWord {
    address: u16,
    data: u32,
}

struct Mem<T> {
    mem: Vec<T>,
}

impl<T> Mem<T> {
    fn new() -> Mem<T> {
        Mem { mem: Vec::new() }
    }

    fn push(&mut self, word: T) {
        self.mem.push(word);
    }

    fn len(&self) -> usize {
        self.mem.len()
    }
}

struct Microcode<'a> {
    file: &'a File,
    version: u16,
    comment: String,
    a_mem: Mem<ABWord>,
    b_mem: Mem<ABWord>,
    c_mem: Mem<CWord>,
    type_map: Mem<TypeWord>,
    pico_store: Mem<PicoStoreWord>,
}

impl<'a> fmt::Display for Microcode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "valid ucode\n\
             version=0x{:04x}\n\
             comment='{}'\n\
             a-mem length={}\n\
             b-mem length={}\n\
             c-mem length={}\n\
             type-map length={}\n\
             pico-store length={}\n\n\
             CMEM:\n\
             {:?}",
            self.version,
            self.comment,
            self.a_mem.len(),
            self.b_mem.len(),
            self.c_mem.len(),
            self.type_map.len(),
            self.pico_store.len(),
            self.c_mem,
        )
    }
}

impl fmt::Display for ABWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04x}: {:010x}", self.address, self.data)
    }
}

/// Control Memory Word Display
impl fmt::Display for CWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:05o}> {:022o} {:022o}",
            self.address,
            self.data_h,
            self.data_l
        )
    }
}

impl fmt::Debug for CWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:05o}>\n{:?}",
            self.address,
            MicroInstruction::new(self)
        )
    }
}

/// Verbose debugging for Control Memory Words
impl fmt::Debug for MicroInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "U AMRA:          {:04o}\n\
             > U R BASE:      {:o}\n\
             A AMRA SEL:      {:o}\n\
             U XYBUS SEL:     {:o}\n\
             U STKP CNT:      {:o}\n\
             U AMWA:          {:04o}\n\
             > LBUS DEV:      {:04o}\n\
             > U W BASE:      {:o}\n\
             > U STKP DIR:    {:o}\n\
             U AMWA SEL:      {:o}\n\
             U SEQ:           {:o}\n\
             U BMRA:          {:03o}\n\
             U BMWA:          {:02o}\n\
             BMEM FROM XBUS:  {:o}\n\
             U MEM:           {:o}\n\
             U SPEC:          {:02o}\n\
             U MAGIC:         {:02o}\n\
             U COND SEL:      {:02o}\n\
             U COND FUNC:     {:o}\n\
             U ALU            {:02o}\n\
             U BYTE F         {:o}\n\
             U OBUS CDR       {:o}\n\
             U OBUS HTYPE     {:o}\n\
             U OBUS LTYPE SEL {:o}\n\
             U CPC SEL        {:o}\n\
             U NPC SEL        {:o}\n\
             U NAF            {:05o}\n\
             U SPEED          {:o}\n\
             U TYPE MAP SEL   {:02o}\n\
             U AU OP          {:03o}\n",

            self.u_amra,
            self.u_r_base,
            self.u_amra_sel,
            self.u_xybus_sel,
            self.u_stkp_count,
            self.u_amwa,
            self.lbus_dev,
            self.u_w_base,
            self.u_stkp_count_dir,
            self.u_amwa_sel,
            self.u_seq,
            self.u_bmra,
            self.u_bmwa,
            self.u_bmem_from_xbus,
            self.u_mem,
            self.u_spec,
            self.u_magic,
            self.u_cond_sel,
            self.u_cond_func,
            self.u_alu,
            self.u_byte_f,
            self.u_obus_cdr,
            self.u_obus_htype,
            self.u_obus_ltype_sel,
            self.u_cpc_sel,
            self.u_npc_sel,
            self.u_naf,
            self.u_speed,
            self.u_type_map_sel,
            self.u_au_op
        )
    }
}

impl fmt::Display for Mem<ABWord> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v: Vec<String> = self.mem.iter().map(|ref a| format!("{}", a)).collect();
        write!(f, "{}", v.join("\n"))
    }
}

impl fmt::Display for Mem<CWord> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v: Vec<String> = self.mem.iter().map(|ref a| format!("{}", a)).collect();
        write!(f, "{}", v.join("\n"))
    }
}

impl fmt::Debug for Mem<CWord> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v: Vec<String> = self.mem.iter().map(|ref a| format!("{:?}", a)).collect();
        write!(f, "{}", v.join("\n"))
    }
}

/// Control store for Microcode
impl<'a> Microcode<'a> {
    fn new(file: &'a File) -> Microcode<'a> {
        Microcode {
            file: file,
            version: 0,
            comment: String::new(),
            a_mem: Mem::new(),
            b_mem: Mem::new(),
            c_mem: Mem::new(),
            type_map: Mem::new(),
            pico_store: Mem::new(),
        }
    }

    fn load(&mut self) -> Result<(), MicrocodeError> {
        self.read_header()?;
        self.read_version()?;
        self.read_comment()?;
        self.read_a_mem()?;
        self.read_b_mem()?;
        self.read_c_mem()?;
        self.read_type_map()?;
        self.read_pico_store_or_eof()?;

        println!("{}", self);

        Ok(())
    }

    /// Read and validate the microcode header.
    fn read_header(&mut self) -> Result<(), MicrocodeError> {
        // Grab the section ID
        let sec = read_u8!(self.file);
        if sec != SEC_HEADER {
            return Err(MicrocodeError::InvalidHeader);
        }

        // Grab the header magic number
        let header = read_u8!(self.file);
        if header != HEADER_MAGIC {
            return Err(MicrocodeError::InvalidHeader);
        }

        Ok(())
    }

    /// Read the microcode version
    fn read_version(&mut self) -> Result<(), MicrocodeError> {
        // Grab the section ID
        let sec = read_u8!(self.file);
        if sec != SEC_VERSION {
            return Err(MicrocodeError::InvalidVersion);
        }

        // Grab the version
        self.version = read_u16!(self.file);

        Ok(())
    }

    /// Read the microcode text comment
    fn read_comment(&mut self) -> Result<(), MicrocodeError> {
        let mut sec = [0; 1];
        self.file.read_exact(&mut sec)?;
        if sec[0] != SEC_COMMENT {
            return Err(MicrocodeError::InvalidComment);
        }

        let mut len_buf = [0; 1];
        self.file.read_exact(&mut len_buf)?;

        let mut char_buf = [0; 1];
        for _ in 0..len_buf[0] as usize {
            self.file.read_exact(&mut char_buf)?;
            self.comment.push(char_buf[0] as char);
        }

        Ok(())
    }

    /// Read A Memory
    fn read_a_mem(&mut self) -> Result<(), MicrocodeError> {
        self.read_a_or_b_mem(SEC_AMEM)
    }

    /// Read B Memory
    fn read_b_mem(&mut self) -> Result<(), MicrocodeError> {
        self.read_a_or_b_mem(SEC_BMEM)
    }

    /// Common function used by A and B Memory reads
    fn read_a_or_b_mem(&mut self, mic_sec: u8) -> Result<(), MicrocodeError> {
        let sec = read_u8!(self.file);
        if sec != mic_sec {
            return Err(MicrocodeError::InvalidABMem);
        }

        loop {
            let count = read_u16!(self.file);

            // A 0 count marks the end of A or B memory
            if count == 0 {
                break;
            }

            let start = read_u16!(self.file);

            for i in 0..count {
                if mic_sec == SEC_AMEM {
                    self.a_mem.push(read_abword!(start + i, self.file));
                } else {
                    self.b_mem.push(read_abword!(start + i, self.file));
                }
            }
        }

        Ok(())
    }

    /// Read C Memory
    fn read_c_mem(&mut self) -> Result<(), MicrocodeError> {
        let sec = read_u8!(self.file);
        if sec != SEC_CMEM {
            return Err(MicrocodeError::InvalidCMem);
        }

        loop {
            let count = read_u16!(self.file);
            // A 0 count marks the end of C memory
            if count == 0 {
                break;
            }

            let start = read_u16!(self.file);

            for i in 0..count {
                self.c_mem.push(read_cword!(start + i, self.file));

                // But there's more!

                loop {
                    // Just consume them for now.
                    let code = read_u8!(self.file);
                    // Done reading extra bytes
                    if code == 0 {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Read the type map
    fn read_type_map(&mut self) -> Result<(), MicrocodeError> {
        let sec = read_u8!(self.file);
        if sec != SEC_TYPEMAP {
            return Err(MicrocodeError::InvalidTypeMap);
        }

        let ntypes = read_u16!(self.file);
        let pad = read_u16!(self.file);
        if pad != 0 {
            return Err(MicrocodeError::InvalidTypeMap);
        }

        for _ in 0..ntypes {
            self.type_map.push(TypeWord { data: read_u8!(self.file) });
        }

        let type_map_end = read_u16!(self.file);
        if type_map_end != 0 {
            return Err(MicrocodeError::InvalidTypeMap);
        }

        Ok(())
    }

    /// Read optional FPA Pico Store.
    ///
    /// This is a special case, likely not present in most microcode,
    /// so we also account for End-of-File here.
    ///
    fn read_pico_store_or_eof(&mut self) -> Result<(), MicrocodeError> {
        // Pico-Store is a special case. If we read an 8 we're done.
        // If we read a 10 we're not done.

        let sec = read_u8!(self.file);
        if sec == SEC_EOF {
            return Ok(());
        } else if sec != SEC_PICOSTORE {
            return Err(MicrocodeError::InvalidPicoStore);
        }

        for _ in 0..255 {
            self.pico_store.push(read_pico_store_word!(self.file));
        }

        let eos = read_u16!(self.file);
        if eos != 0xffff {
            return Err(MicrocodeError::InvalidPicoStore);
        }

        let eof = read_u8!(self.file);
        if eof != SEC_EOF {
            return Err(MicrocodeError::InvalidPicoStoreEof);
        }

        Ok(())
    }
}

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

    let mut state = Microcode::new(&file);

    match state.load() {
        Ok(()) => (),
        Err(reason) => println!("Unable to parse microcode: {}", reason),
    }
}
