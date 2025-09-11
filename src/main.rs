//! Bookshelf sample reader
//! Simple main program to demonstrate things.
//!
pub mod bookshelf;
pub mod marklist;

use std::path::Path;

use argh::FromArgs;
use bookshelf_r::bookshelf::{PinDetail, PinInstance};
#[derive(FromArgs)]
/// Bookshelf template reader
struct Args {
    /// aux file
    #[argh(option, short = 'a')]
    aux: Option<String>,

    /// block packing
    #[argh(switch, short = 'b')]
    block: bool,

    /// cell wire length
    #[argh(option, short = 'c')]
    cell: Option<String>,

    /// net wire length
    #[argh(option, short = 'n')]
    net: Option<String>,

    /// alternate PL file
    #[argh(option, short = 'p')]
    plfile: Option<String>,

    /// output PL file
    #[argh(option, short = 'o')]
    output_pl: Option<String>,

    /// postscript file name
    #[argh(option, short = 'P')]
    postscript: Option<String>,

    /// flip demo
    #[argh(switch, short = 'f')]
    flipdemo: bool,

    /// export AUX and associated files
    #[argh(option, short = 'x')]
    export: Option<String>
}

fn main() {
    println!("Main program for bookshelf reader.\n");

    let arguments: Args = argh::from_env();

    if arguments.flipdemo {
        println!("Flip demo.");
        flipdemo();
        return;
    }

    let auxname;
    match arguments.aux {
        Some(b) => {
            auxname = b;
        }
        _ => {
            println!("Specify a Bookshelf file name");
            return;
        }
    }

    let mut bc;
    if !arguments.block {
        println!("Bookshelf Standard Cell/Mixed Size reader");
        bc = bookshelf::BookshelfCircuit::read_aux(&auxname.clone());
        if arguments.plfile.is_some() {
            let f = arguments.plfile.unwrap();
            let path = Path::new(&f);
            bc.read_pl(path, false);
        }
        bc.summarize();
    } else {
        println!("Bookshelf Block Packing Reader");
        bc = bookshelf::BookshelfCircuit::read_blockpacking(auxname);
        bc.summarize();
    }
    if arguments.cell.is_some() {
        let mut wlc = bookshelf::WlCalc::new(&bc);
        let cidx = bc.cell_index(&arguments.cell.unwrap().clone()).unwrap();
        println!("Cell index {} is {}", cidx, bc.cells[cidx].name);
        println!(
            "Cell is {} by {} located at ({}, {}) orientation {}",
            bc.cells[cidx].w, bc.cells[cidx].h, bc.cellpos[cidx].x, bc.cellpos[cidx].y, bc.orient[cidx],
        );
        wlc.add_cells(&bc, &vec![cidx]);
        for p in &bc.cells[cidx].pins {
            println!(
                "  Pin {} at dxdy {} {}    Net {} {}  wire length {}",
                p.name, p.dx, p.dy,
                p.parent_net,
                bc.nets[p.parent_net].name,
                bc.net_wl(&bc.nets[p.parent_net])
            );
        }
        println!("Connected net wire length {}", wlc.wl(&bc));
    }
    if arguments.net.is_some() {
        let nidx = bc.net_index(&arguments.net.unwrap().clone()).unwrap();

        println!(
            "Net {} {} wire length {}",
            nidx,
            bc.nets[nidx].name,
            bc.net_wl(&bc.nets[nidx])
        );
        for pr in &bc.nets[nidx].pins {
            let instance = &bc.cells[pr.parent_cell].pins[pr.index];
            println!(
                "  Cell {} at {} {}, orientation {}, pin {} index {} offset {} {}",
                bc.cells[pr.parent_cell].name, bc.cellpos[pr.parent_cell].x, bc.cellpos[pr.parent_cell].y,
                bc.orient[pr.parent_cell],
                instance.name,
                pr.index, instance.dx, instance.dy
            );
        }
    }
    if arguments.postscript.is_some() {
        bc.postscript(arguments.postscript.unwrap());
    }
    if arguments.output_pl.is_some() {
        bc.write_pl(arguments.output_pl.unwrap().clone(), & bc.notes);
    }
    if arguments.export.is_some() {
        bc.write_aux(&arguments.export.unwrap());
    }
}

use bookshelf_r::bookshelf::Cell;
use bookshelf_r::bookshelf::Orientation;
use pstools::PSTool;

fn show_cell(cell: &mut Cell, orient: Orientation, offset: f32, pst: &mut PSTool) {
    bookshelf_r::bookshelf::BookshelfCircuit::orient_cell(cell, orient);

    pst.add_box(offset, 0.0, offset + cell.w, cell.h);
    println!("New orientation: {}", orient);
    println!("WH now: {} {}", cell.w, cell.h);
    pst.add_text(offset + cell.w * 0.5, cell.h * 0.5, format!("{}", orient));
    for p in &cell.pins {
        println!(
            "  {}   {} {}  -> {} {}",
            p.name, p.details[0].dx, p.details[0].dy, p.dx, p.dy
        );
        pst.add_text(offset + p.dx, p.dy, p.name.clone());
    }
    for i in 1..cell.pins.len() {
        let p_old = &cell.pins[i - 1];
        let p_new = &cell.pins[i];
        pst.add_line(offset + p_old.dx, p_old.dy, offset + p_new.dx, p_new.dy);
    }
}

pub fn flipdemo() {
    let mut c = bookshelf_r::bookshelf::Cell {
        w: 100.0,
        h: 200.0,
        name: "test.".to_string(),
        original_w: 100.0,
        original_h: 200.0,
        pins: Vec::new(),
        terminal: false,
        soft: None,
        is_macro: false,
        can_rotate: true,
    };
    c.pins.push(PinInstance {
        name: "a".to_string(),
        dx: 6.0,
        dy: 6.0,
        parent_cell: 0,
        parent_net: 0,
        details: vec![PinDetail { dx: 6.0, dy: 6.0 }],
    });
    c.pins.push(PinInstance {
        name: "b".to_string(),
        dx: 6.0,
        dy: 195.0,
        parent_cell: 0,
        parent_net: 0,
        details: vec![PinDetail { dx: 36.0, dy: 145.0 }],
    });
    c.pins.push(PinInstance {
        name: "c".to_string(),
        dx: 95.0,
        dy: 195.0,
        parent_cell: 0,
        parent_net: 0,
        details: vec![PinDetail {
            dx: 95.0,
            dy: 195.0,
        }],
    });
    let mut or = Vec::new();
    or.push(Orientation::N);
    or.push(Orientation::FN);
    or.push(Orientation::S);
    or.push(Orientation::FS);
    or.push(Orientation::E);
    or.push(Orientation::FE);
    or.push(Orientation::W);
    or.push(Orientation::FW);

    let mut pst = pstools::PSTool::new();
    let mut offset = 0.0;
    for orient in &or {
        show_cell(&mut c, *orient, offset, &mut pst);
        offset = offset + 210.0;
    }
    pst.set_border(40.0);
    pst.generate("rotations.ps".to_string()).unwrap();
}
