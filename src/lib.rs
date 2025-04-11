//! # Bookshelf_r
//!
//! `bookshelf_r` is a library to read GSRC format circuit
//! descriptions.  There is typically an *aux* file that
//! lists the nodes, nets, and standard cell rows, along with
//! a few other files to describe the circuit.
//!
//! Circuit designs are loaded into a *BookshelfCircuit* structure,
//! with public fields so that the nets, cells, and pins, can be
//! examined and manipulated.
//!
//! There is also a *marklist* library, useful for tagging cells
//! and nets.  The primary use of a marklist is in the construction
//! of a hypergraph for a subset of cells -- it's necessary to
//! determine the set of nets connected to a set of cells, and
//! each net needs to be identified once (and only once).  The
//! marklist helps make this process more efficient.
//!
//! The hypergraph structures are contained in the external
//! `metapartition` crate.
pub mod bookshelf;
pub mod marklist;
pub extern crate metapartition;

// pub mod hypergraph;
// pub mod bbox;  // Now in pstools
// pub mod point;
