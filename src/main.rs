// use Bookshelf;
use bookshelf::BookshelfCircuit;
use std::env;
// use malduit::optimizer;

fn main() {
    println!("Main program for bookshelf reader.\n");
    let args: Vec<String> = env::args().collect();

    // optimizer::maltest();

    if args.len() == 2 {
        let mut bc = BookshelfCircuit::read_aux(args[1].clone());
	bc.summarize();
    }
}
