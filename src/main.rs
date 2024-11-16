// use Bookshelf;
// use crate::lib::BookshelfCircuit;
// use bookshelf::BookshelfCircuit;
use std::env;

// use malduit::optimizer;
// use bookshelf_r;
// use crate::bookshelf::BookshelfCircuit;
pub mod bookshelf;
pub mod marklist;
pub mod bbox;
pub mod point;

fn main() {
    println!("Main program for bookshelf reader.\n");
    let args: Vec<String> = env::args().collect();

    // optimizer::maltest();

    let mut ml = marklist::MarkList::new(40);
    // marklist::ml::mark(4);
    ml.mark(4);
    ml.mark(3);
    ml.mark(4);
    ml.mark(3);
    ml.dump();
    ml.clear();
    ml.mark(33);
    ml.mark(4);
    ml.dump();
    
    if args.len() == 2 {
        let bc = crate::bookshelf::BookshelfCircuit::read_aux(args[1].clone());
        bc.summarize();

        let wl = bc.wl();
        println!("Wire length {}", wl);
    }
}
