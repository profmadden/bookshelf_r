// use Bookshelf;
// use crate::lib::BookshelfCircuit;
// use bookshelf::BookshelfCircuit;
use std::env;

// use malduit::optimizer;
// use bookshelf_r;
// use crate::bookshelf::BookshelfCircuit;
pub mod bookshelf;
pub mod marklist;
use pstools_r::bbox;
use pstools_r::point;

use argh::FromArgs;
#[derive(FromArgs)]
/// Bookshelf template reader
struct Args {
    /// aux file
    #[argh(option, short = 'a')]
    aux: Option<String>,

    /// block packing
    #[argh(switch, short = 'b')]
    block: bool,
}

fn main() {
    println!("Main program for bookshelf reader.\n");
    
    let arguments: Args = argh::from_env();
    let auxname;
    match arguments.aux {
        Some(b) => {
            auxname = b;
        },
        _ => {println!("Specify a Bookshelf file name"); return;},
    }


    if !arguments.block {
        let bc = crate::bookshelf::BookshelfCircuit::read_aux(auxname.clone());
        bc.summarize();

        let wl = bc.wl();
        println!("Wire length {}", wl);
        bc.postscript("standardcell.ps".to_string());
    } else {
        println!("Read input as block packing.");
        let bc = crate::bookshelf::BookshelfCircuit::read_blockpacking(auxname);
        bc.summarize();
        bc.postscript("blockplacement.ps".to_string());
        // for i in 0..bc.cells.len() {
            // println!("Cell {} at {} {}", bc.cells[i].name, bc.cellpos[i].x, bc.cellpos[i].y);
        // }
    }
}
