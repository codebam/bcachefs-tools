use clap::Parser;
use std::io::{BufRead, Write};

use bch_bindgen::bcachefs;
use bch_bindgen::c;
use bch_bindgen::fs::Fs;

mod bkey_types;
mod parser;

use bch_bindgen::c::bpos;

/// Debug a bcachefs filesystem.
#[derive(Parser, Debug)]
pub struct Cli {
    #[arg(required(true))]
    devices: Vec<std::path::PathBuf>,

    #[arg(short, long)]
    command: Option<String>,
}

#[derive(Debug)]
enum DebugCommand {
    Dump(DumpCommand),
    Update(UpdateCommand),
}

#[derive(Debug)]
struct DumpCommand {
    btree: String,
    bpos: bpos,
}

#[derive(Debug)]
struct UpdateCommand {
    btree: String,
    bpos: bpos,
    bkey: String,
    field: String,
    value: u64,
}

fn update(fs: &Fs, type_list: &bkey_types::BkeyTypes, cmd: UpdateCommand) {
    let id: bch_bindgen::c::btree_id = cmd.btree.parse().expect("no such btree");

    let (bkey, inode_bkey) = if cmd.bkey == "bch_inode_unpacked" {
        (c::bch_bkey_type::KEY_TYPE_MAX, true)
    } else {
        (cmd.bkey["bch_".len()..].parse().expect("no such bkey"), false)
    };

    if let Some((size, offset)) = type_list.get_member_layout(&cmd.bkey, &cmd.field) {
        let update = c::bkey_update {
            id,
            bkey,
            inode_bkey,
            offset,
            size,
            value: cmd.value,
        };
        unsafe {
            c::cmd_update_bkey(fs.raw, update, cmd.bpos);
        }
    } else {
        println!("unknown field '{}'", cmd.field);
    }
}

fn dump(fs: &Fs, cmd: DumpCommand) {
    let id: bch_bindgen::c::btree_id = cmd.btree.parse().expect("no such btree");

    unsafe {
        c::cmd_dump_bkey(fs.raw, id, cmd.bpos);
    }
}

fn usage() {
    println!("Usage:");
    println!("    dump <btree_type> <bpos>");
    println!("    update <btree_type> <bpos> <bkey_type>.<field>=<value>");
}

fn do_command(fs: &Fs, type_list: &bkey_types::BkeyTypes, cmd: &str) -> i32 {
    match parser::parse_command(cmd) {
        Ok(cmd) => {
            match cmd {
                    DebugCommand::Dump(cmd) => dump(fs, cmd),
                    DebugCommand::Update(cmd) => update(fs, type_list, cmd),
            };

            0
        }
        Err(e) => {
            println!("{e}");
            usage();

            1
        }
    }
}

pub fn debug(argv: Vec<String>) -> i32 {
    fn prompt() {
        print!("bcachefs> ");
        std::io::stdout().flush().unwrap();
    }

    let opt = Cli::parse_from(argv);
    let fs_opts: bcachefs::bch_opts = Default::default();
    let type_list = bkey_types::get_bkey_type_info();

    if let Some(cmd) = opt.command {
        return match parser::parse_command(&cmd) {
            Ok(cmd) => {
                let fs = match Fs::open(&opt.devices, fs_opts) {
                    Ok(fs) => fs,
                    Err(_) => {
                        return 1;
                    }
                };
                match cmd {
                    DebugCommand::Dump(cmd) => dump(&fs, cmd),
                    DebugCommand::Update(cmd) => update(&fs, &type_list, cmd),
                }

                0
            }
            Err(e) => {
                println!("{e}");
                usage();

                1
            }
        };
    }

    let fs = match Fs::open(&opt.devices, fs_opts) {
        Ok(fs) => fs,
        Err(_) => {
            return 1;
        }
    };

    prompt();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        do_command(&fs, &type_list, &line.unwrap());
        prompt();
    }

    0
}

pub fn list_bkeys() -> i32 {
    print!("{}", bkey_types::get_bkey_type_info());

    0
}