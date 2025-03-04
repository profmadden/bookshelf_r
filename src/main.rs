/// Bookshelf sample reader
pub mod bookshelf;
pub mod marklist;

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

    /// cell wire length
    #[argh(option, short = 'c')]
    cell: Option<String>,

    /// net wire length
    #[argh(option, short = 'n')]
    net: Option<String>,

    /// postscript file name
    #[argh(option, short = 'p')]
    postscript: Option<String>,
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

    let bc;
    if !arguments.block {
        println!("Bookshelf Standard Cell/Mixed Size reader");
        bc = bookshelf::BookshelfCircuit::read_aux(&auxname.clone());
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
        println!("Cell is {} by {} located at ({}, {})", bc.cells[cidx].w, bc.cells[cidx].h, bc.cellpos[cidx].x, bc.cellpos[cidx].y);
        wlc.add_cells(&bc, &vec![cidx]);
        for p in &bc.cells[cidx].pins {
            println!("  Net {} {}  wire length {}", p.parent_net, bc.nets[p.parent_net].name, bc.net_wl(&bc.nets[p.parent_net]));
        }
        println!("Connected net wire length {}", wlc.wl(&bc));
    }
    if arguments.net.is_some() {
        let nidx = bc.net_index(&arguments.net.unwrap().clone()).unwrap();

        println!("Net {} {} wire length {}", nidx, bc.nets[nidx].name, bc.net_wl(&bc.nets[nidx]));
        for pr in &bc.nets[nidx].pins {
            let instance = &bc.cells[pr.parent_cell].pins[pr.index];
            println!("  Cell {} index {} is pin offset {} {}", bc.cells[pr.parent_cell].name, pr.index, instance.dx, instance.dy);
        }
    }
    if arguments.postscript.is_some() {
        bc.postscript(arguments.postscript.unwrap());
    }
}
