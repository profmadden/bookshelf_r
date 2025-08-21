//! The Bookshelf library contains functions to read and write
//! GSRC bookshelf format circuits descriptions.  This is a
//! fairly simple text format used in many academic research
//! projects.
//!
//! The BookshelfCircuit struct contains vectors of cells,
//! and nets.  Cells contain PinInstnaces (the actual
//! pin itself, with links to the parent cell and parent net).
//! Nets contain PinRefs, which indicate the cell that a
//! pin belongs to, along with the index into the list of
//! pins on that cell.
//!
//! Usize variables are used to index into the vectors.  The
//! BookshelfCircuit structure also contains hash maps, enabling
//! a lookup for the cell or net index from a string name.
//!
//! The library utilizes metapartition -- to construct
//! hypergraphs for a portion of a circuit, as needed.
extern crate libc;
use libc::c_char;
use std::cmp;
use std::collections::HashMap;
use std::ffi::CStr;

use scan_fmt::scan_fmt;
use sscanf::sscanf;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;

use pstools::PSTool;

const LDBG: bool = false;

// mod crate::bbox;
// mod point;
use pstools::bbox;
use pstools::point;

// use crate::point;

use pstools;
use std::fmt;

use hypergraph::hypergraph;

/// PinInstances are in the vector for the cells
#[derive(Clone)]
pub struct PinInstance {
    pub name: String,
    pub dx: f32,
    pub dy: f32,
    pub parent_cell: usize,
    pub parent_net: usize,
    pub details: Vec<PinDetail>,
}

/// PinDetail has the original (non-rotated/translated)
/// locations for each pin of a cell.  There may be multiple
/// electrically equivalent pins; one of these is promoted
/// and set for the cell itself.  
#[derive(Copy, Clone)]
pub struct PinDetail {
    pub dx: f32,
    pub dy: f32,
}

// PinRefs are in the vector for the nets
#[derive(Copy, Clone)]
pub struct PinRef {
    pub parent_cell: usize,
    pub index: usize,
}

pub struct Orientation {
    pub orient: u8,
}

pub struct AltSize {
    pub w: f32,
    pub h: f32,
}

/// Some blocks and macros are soft, and can be
/// resized.  Within a cell structure, the W and H
/// fields are the size of any fixed instance,
/// but it's possible to swap these out for
/// alternative dimensions.
///
/// If the block is not marked soft, then the
/// sizes listed within the alt_sizes vector are the
/// only ones available.  If the block is soft, the
/// width and height of the orignal block (which is
/// the required area) can be used to generate
/// alternate sizes for the block.
///
/// In the kujenga floor planner, an incoming block
/// may have multiple sizes.  This allows reading in
/// of a block packing file using the bookshelf circuit
/// structure, and then generating a variety of block
/// sizes when the floor planner runs.
pub struct SoftSize {
    pub soft: bool,
    pub min_aspect: f32,
    pub max_aspect: f32,
    pub alt_sizes: Vec<AltSize>,
}

pub struct Cell {
    pub name: String,
    pub w: f32,
    pub h: f32,
    // pub x: f32,
    // pub y: f32,
    pub pins: Vec<PinInstance>,
    pub terminal: bool,
    pub soft: Option<SoftSize>,
    pub is_macro: bool,
    pub can_rotate: bool,
}

impl Cell {
    pub fn area(&self) -> f32 {
        self.h * self.w
    }
}

pub struct Net {
    pub name: String,
    pub pins: Vec<PinRef>,
}

pub struct Macro {
    pub name: String,
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
    pub pins: Vec<PinInstance>,
}

pub struct Row {
    pub name: String,
    pub bounds: bbox::BBox,
    pub site_spacing: f32,
}
pub struct Testme {
    pub p: point::Point,
    pub v: f32,
}

/// A BookshelfCircuit contains the specifications for a
/// circuit design -- cells, nets, and so on.  Cell positions
/// are located in the cellpos field, and not directly with
/// the cell, as they change often.  refpos is similar to
/// cellpos, and provides reference point information (for
/// use in tracking abstract-to-legal placements, or in
/// comparing two different placements of the same circuit).
///
/// Cells and nets each have a unique ID (the index into
/// the cells and nets arrays, respectively).  The BookshelfCircuit
/// struct also has maps for finding the index of any text-named
/// cell or net.
///
/// A notes field is included (for annotations that would appear
/// in an output .pl file, or on a visual rendering of the circuit).
/// Initial notes are created with the files are read in (aux file name,
/// and the .pl file name).
/// Command line arguments, date, and so on, are also reasonable notes to
/// add.
pub struct BookshelfCircuit {
    pub counter: i32,
    pub name: String,
    pub cells: Vec<Cell>,
    pub cellpos: Vec<point::Point>,
    /// Optional cell coloring scheme.  If present, each cell/macro
    /// will be colored based on the index -- will need to implement
    /// a basic color selection mechanism.
    pub cell_color: Option<Vec<usize>>,
    /// Optional reference position
    pub refpos: Option<Vec<point::Point>>,
    pub orient: Vec<Orientation>,
    pub nets: Vec<Net>,
    pub macros: Vec<Macro>,
    pub rows: Vec<Row>,
    pub cell_map: HashMap<String, usize>,
    pub net_map: HashMap<String, usize>,
    pub macro_map: HashMap<String, usize>,
    pub notes: Vec<String>,
    pub unit_x: f32,
    pub unit_y: f32,
    pub num_macros: usize,
    pub num_terminals: usize,
    pub num_cells: usize,
    pub row_height: f32,
}

pub struct WlCalc {
    pub marked_nets: MarkList,
}

impl WlCalc {
    pub fn new(bc: &BookshelfCircuit) -> WlCalc {
        WlCalc {
            marked_nets: MarkList::new(bc.nets.len()),
        }
    }
    pub fn add_cells(&mut self, bc: &BookshelfCircuit, cells: &Vec<usize>) {
        for c in cells {
            for p in &bc.cells[*c].pins {
                self.marked_nets.mark(p.parent_net);
            }
        }
    }
    pub fn clear(&mut self) {
        self.marked_nets.clear();
    }
    pub fn wl(&self, bc: &BookshelfCircuit) -> f32 {
        let mut total = 0.0;
        for n in &self.marked_nets.list {
            total = total + bc.net_wl(&bc.nets[*n]);
        }

        total
    }
}

impl fmt::Display for BookshelfCircuit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "name {} cells, {} nets, core {}, HPWL {}",
            self.cells.len(),
            self.nets.len(),
            self.core(),
            self.wl()
        )
    }
}

impl BookshelfCircuit {
    pub fn new() -> BookshelfCircuit {
        let bc = BookshelfCircuit {
            counter: 0,
            name: "bookshelf_circuit".to_string(),
            cells: Vec::new(),
            cellpos: Vec::new(),
            cell_color: None,
            refpos: None,
            orient: Vec::new(),
            nets: Vec::new(),
            macros: Vec::new(),
            rows: Vec::new(),
            cell_map: HashMap::new(),
            net_map: HashMap::new(),
            macro_map: HashMap::new(),
            notes: Vec::new(),
            unit_x: 1.0,
            unit_y: 1.0,
            num_cells: 0,
            num_macros: 0,
            num_terminals: 0,
            row_height: 0.0,
        };

        bc
    }
    pub fn ps_terminals(&self, pst: &mut PSTool) {
        // Terminals n the background
        pst.set_color(1.0, 0.3, 0.3, 1.0);
        for i in 0..self.cells.len() {
            if self.cells[i].terminal {
                // pst.add_text(self.cellpos[i].x, self.cellpos[i].y, self.cells[i].name.clone());
                pst.add_box(
                    self.cellpos[i].x - 1.0,
                    self.cellpos[i].y - 1.0,
                    self.cellpos[i].x + self.cells[i].w + 1.0,
                    self.cellpos[i].y + self.cells[i].h + 1.0,
                );
            }
        }
    }
    pub fn ps_cells(&self, pst: &mut PSTool) {
        pst.set_color(0.1, 0.1, 0.8, 1.0);
        for i in 0..self.cells.len() {
            if !self.cells[i].terminal {
                // pst.add_text(self.cellpos[i].x, self.cellpos[i].y, self.cells[i].name.clone());
                pst.add_box(
                    self.cellpos[i].x + 0.25,
                    self.cellpos[i].y + 0.25,
                    self.cellpos[i].x + self.cells[i].w - 0.5,
                    self.cellpos[i].y + self.cells[i].h - 0.5,
                );
            }
        }
    }
    pub fn ps_labels(&self, pst: &mut PSTool) {
        pst.set_color(0.1, 0.1, 0.0, 1.0);
        for i in 0..self.cells.len() {
            if !self.cells[i].terminal {
                pst.add_text(
                    self.cellpos[i].x + 1.0,
                    self.cellpos[i].y + 1.0,
                    self.cells[i].name.clone(),
                );
            }
        }
    }
    pub fn ps_movement(&self, pst: &mut PSTool) {
        if self.refpos.is_none() {
            return;
        }

        pst.set_color(1.0, 0.0, 0.0, 1.0);
        let rp = self.refpos.as_deref().unwrap();

        for i in 0..self.cells.len() {
            // let rp: Option<pstools::point::Point> = self.refpos.as_ref()[i];
            pst.add_line(self.cellpos[i].x, self.cellpos[i].y, rp[i].x, rp[i].y);
        }
    }
    pub fn postscript(&self, filename: String) {
        let mut pst = pstools::PSTool::new();

        self.ps_terminals(&mut pst);
        self.ps_cells(&mut pst);
        self.ps_terminals(&mut pst);

        pst.set_border(40.0);
        pst.generate(filename).unwrap();
    }

    pub fn postscript_wl(&self, filename: String) {
        let mut impact: Vec<f32> = vec![0.0 as f32; self.cells.len()];
        for c in 0..self.cells.len() {
            for p in &self.cells[c].pins {
                impact[c] += self.net_wl(&self.nets[p.parent_net]);
            }
        }
        let wl = self.wl();
        for c in 0..self.cells.len() {
            impact[c] = (impact[c] / wl) / self.cells[c].area();
        }
        let mut max = impact[0];
        let mut min = impact[0];
        for c in 0..self.cells.len() {
            max = max.max(impact[c]);
            min = min.min(impact[c]);
        }

        let range = max - min;
        println!("WL contribution range {} to {}", max, min);

        let mut pst = pstools::PSTool::new();
        pst.set_fill(true);
        for c in 0..self.cells.len() {
            let color = (impact[c] - min) / range;
            pst.set_color(color, 0.0, 1.0 - color, 1.0);
            pst.add_box(
                self.cellpos[c].x,
                self.cellpos[c].y,
                self.cellpos[c].x + self.cells[c].w,
                self.cellpos[c].y + self.cells[c].h,
            );
        }

        pst.set_border(40.0);
        pst.generate(filename).unwrap();
    }

    pub fn cellweights(&self, cells: &Vec<usize>) -> f32 {
        let mut total = 0.0;
        for cell_id in cells {
            total = total + self.cells[*cell_id].area();
        }
        total
    }

    // Set the cell position -- the point is the center, we adjust for lower left
    pub fn set_cell_center(&mut self, cid: usize, loc: &point::Point) {
        self.cellpos[cid].x = loc.x - self.cells[cid].w / 2.0;
        self.cellpos[cid].y = loc.y - self.cells[cid].h / 2.0;
    }

    pub fn set_cell_centers(&mut self, cells: &Vec<usize>, loc: &point::Point) {
        for cid in cells {
            self.set_cell_center(*cid, loc);
        }
    }

    pub fn read_aux(filename: &String) -> BookshelfCircuit {
        let f = File::open(filename.clone()).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();

        if LDBG {
            println!("Returned line {}", line);
        }

        let (nodef, netf, _wtf, plf, sclf) = scan_fmt!(
            &line,
            "RowBasedPlacement : {} {} {} {} {}",
            String,
            String,
            String,
            String,
            String
        )
        .unwrap();

        println!("Node file {}", nodef);

        let path = Path::new(&filename);

        let mut bc = BookshelfCircuit::new();

        bc.read_nodes(path.with_file_name(nodef).as_path());
        bc.read_nets(path.with_file_name(netf).as_path());
        bc.read_pl(path.with_file_name(plf).as_path(), false);
        bc.read_scl(path.with_file_name(sclf).as_path());
        if bc.rows.len() > 0 {
            bc.unit_x = bc.rows[0].site_spacing;
            bc.unit_y = bc.rows[0].bounds.dy();
            bc.row_height = bc.unit_y;
        }
        // Now go through and classify all the cell types
        bc.num_cells = 0;
        bc.num_macros = 0;
        bc.num_terminals = 0;
        for c in &mut bc.cells {
            if c.terminal {
                bc.num_terminals += 1;
            } else {
                if c.h > bc.row_height {
                    bc.num_macros += 1;
                    c.is_macro = true;
                } else {
                    bc.num_cells += 1;
                    c.is_macro = false;
                }
            }
        }

        println!(
            "Circuit read: {} cells, {} are terminals, {} are macros",
            bc.num_cells, bc.num_terminals, bc.num_macros
        );
        println!("Row height: {}", bc.row_height);

        if LDBG {
            println!("BC counter is {}", bc.counter);
        }

        bc
    }

    pub fn read_nodes(&mut self, filepath: &Path) -> usize {
        println!("Opening {}", filepath.to_string_lossy());

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let _line = BookshelfCircuit::getline(&mut reader).unwrap();
        // println!("First line of nodes file {}", line);

        self.counter = self.counter + 1;

        // Look for the nodes line
        let mut num_node = 0 as i32;
        let mut num_term = 0 as i32;

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nn)) = scan_fmt!(&line, "NumNodes : {d}", i32) {
            if LDBG {
                println!("Scan fmt worked, value is {}", nn);
            }
            num_node = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nt)) = scan_fmt!(&line, "NumTerminals : {d}", i32) {
            if LDBG {
                println!("Scan fmt worked, value is {}", nt);
            }
            num_term = nt;
        }

        println!("Nodes file has {} nodes, {} terminals", num_node, num_term);

        self.cells = Vec::with_capacity(num_node as usize);

        for i in 0..num_node {
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            // println!(" > line < {}", line);
            if let Ok((cellname, x, y)) = scan_fmt!(&line, " {} {} {}", String, String, String) {
                // println!(" Scanned cell {}  --> {} size {}x{}", i, cellname, x, y);
                let xf: f32 = x.parse().unwrap();
                let yf: f32 = y.parse().unwrap();
                // println!("  Floating point dimensions: {} {}", xf, yf);

                let mut isterminal = false;
                if line.contains("terminal") {
                    // println!("  -- TERMINAL");
                    isterminal = true;
                }

                let cn = self.find_cell(cellname.clone());
                // println!("Map cell to ID {}", cn);

                let c = Cell {
                    name: cellname,
                    w: xf,
                    h: yf,
                    // x: 0.0,
                    // y: 0.0,
                    pins: Vec::new(),
                    terminal: isterminal,
                    soft: None,
                    is_macro: false,
                    can_rotate: false,
                };

                self.cells.push(c);

                let cp = point::Point {
                    x: 0.0,
                    y: 0.0,
                    // orientation: 0,
                };
                self.cellpos.push(cp);

                let co = Orientation { orient: 0 };
                self.orient.push(co);
            } else {
                println!("Not ok match");
            }
        }

        0
    }

    pub fn write_nodes_fixed(&mut self, filepath: &String) {
        let mut f = File::create(filepath).unwrap();
        writeln!(&mut f, "UCLA nodes 1.0").unwrap();
        writeln!(&mut f, "# Generated by bookshelf_r").unwrap();
        let mut num_fixed = 0;
        for i in 0..self.cells.len() {
            if self.cells[i].is_macro || self.cells[i].terminal {
                num_fixed += 1;
            }
        }
        writeln!(&mut f, "NumNodes : {}", self.cells.len()).unwrap();
        writeln!(&mut f, "NumTerminals : {}", num_fixed).unwrap();
        for c in &self.cells {
            if c.is_macro || c.terminal {
                writeln!(&mut f, "{}  {} {} terminal", c.name, c.w, c.h).unwrap();
            } else {
                writeln!(&mut f, "{}  {} {}", c.name, c.w, c.h).unwrap();
            }
        }

    }

    pub fn write_nets_center(&self, filepath: &String) {
        let mut f = File::create(filepath).unwrap();
        writeln!(&mut f, "UCLA nets 1.0").unwrap();
        writeln!(&mut f, "# Pin Centering from bookshelf_r").unwrap();
        writeln!(&mut f, "NumNets : {}", self.nets.len()).unwrap();
        let mut num_pins = 0;
        for net in &self.nets {
            num_pins += net.pins.len();
        }
        writeln!(&mut f, "NumPins : {}", num_pins).unwrap();
        for net in &self.nets {
            println!("NetDegree : {} {}", net.pins.len(), net.name);
            for p in &net.pins {
                let c = &self.cells[p.parent_cell];
                writeln!(&mut f, " {} B : {} {}", c.name, c.w / 2.0, c.h / 2.0).unwrap();
            }
        }
    }

    pub fn read_nets(&mut self, filepath: &Path) -> usize {
        // println!("Opening {}", filename);

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if LDBG {
            println!("First line of nets file {}", line);
        }

        self.counter = self.counter + 1;

        // Look for the nodes line
        let mut num_nets = 0 as usize;
        let mut num_pins = 0 as usize;

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nn)) = scan_fmt!(&line, "NumNets : {d}", usize) {
            if LDBG {
                println!("Scan fmt worked, value is {}", nn);
            }
            num_nets = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok(np) = scan_fmt!(&line, "NumPins : {d}", usize) {
            if LDBG {
                println!("Scan fmt worked, value is {}", np);
            }
            num_pins = np;
        }

        if LDBG {
            println!("Nets file has {} nets, {} pins", num_nets, num_pins);
        }
        self.nets = Vec::with_capacity(num_nets);

        for nidx in 0..num_nets {
            let line1 = BookshelfCircuit::getline(&mut reader).unwrap();
            // Hack the line format -- block packing examples don't have net names?
            let line = format!("{} _net{}", line1, nidx);

            if let Ok((nd, nn)) = scan_fmt!(&line, "NetDegree : {d} {}", usize, String) {
                if LDBG {
                    println!("Net {} degree {}", nn, nd);
                }
                let netnum = self.find_net(nn.clone());
                let mut net = Net {
                    name: nn.clone(),
                    pins: Vec::with_capacity(nd),
                };

                for p in 0..nd {
                    let mut cellname: String = "".to_string();
                    let mut dx: f32 = 0.0;
                    let mut dy: f32 = 0.0;

                    let line = BookshelfCircuit::getline(&mut reader).unwrap();
                    if LDBG {
                        println!("Pin line {}", line);
                    }

                    if let Ok((cn, sdir, colon, sdx, sdy)) = scan_fmt!(
                        &line,
                        " {} {} {} {} {}",
                        String,
                        String,
                        String,
                        String,
                        String
                    ) {
                        cellname = cn.clone();
                        if LDBG {
                            println!("PIN NAME {} sdx {} sdy {}", cellname, sdx, sdy);
                        }

                        dx = sdx.parse().unwrap();
                        dy = sdy.parse().unwrap();
                    } else if let Ok((cn, sdir)) = scan_fmt!(&line, " {} {}", String, String) {
                        cellname = cn.clone();
                    }

                    if LDBG {
                        println!("Create pin for cell {} at {} {}", cellname, dx, dy);
                    }
                    let cidx = self.find_cell(cellname);

                    let pr = PinRef {
                        parent_cell: cidx,
                        index: self.cells[cidx].pins.len(),
                    };
                    // Move pin offsets so that they are relative to the
                    // lower left corner of a cell.  When cell orientations
                    // are changed, need to take this into account.
                    let offx = self.cells[cidx].w / 2.0;
                    let offy = self.cells[cidx].h / 2.0;

                    net.pins.push(pr);
                    let pi = PinInstance {
                        name: "".to_string(),
                        dx: dx + offx,
                        dy: dy + offy,
                        parent_cell: cidx,
                        parent_net: nidx,
                        details: Vec::new(),
                    };
                    self.cells[cidx].pins.push(pi);
                }
                self.nets.push(net);
            }
        }

        0
    }

    pub fn read_pl(&mut self, filepath: &Path, reference: bool) -> usize {
        // println!("Opening {}", filename);

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if LDBG {
            println!("First line of PL file {}", line);
        }

        let mut refpos = Vec::new();
        if reference {
            refpos = self.cellpos.clone();
        }

        loop {
            let line = BookshelfCircuit::getline(&mut reader);
            match line {
                Ok(l) => {
                    if LDBG {
                        println!("Read PL line {}", l);
                    }
                    if let Ok((cellname, x, y)) = scan_fmt!(&l, " {} {} {}", String, String, String)
                    {
                        let cidx = self.find_cell(cellname.clone());
                        if !reference {
                            self.cellpos[cidx].x = x.parse().unwrap();
                            self.cellpos[cidx].y = y.parse().unwrap();
                            if LDBG {
                                println!(
                                    "  Locate cell {} idx {} at {} {}",
                                    cellname, cidx, self.cellpos[cidx].x, self.cellpos[cidx].y
                                );
                            }
                        } else {
                            refpos[cidx].x = x.parse().unwrap();
                            refpos[cidx].y = y.parse().unwrap();
                        }
                    }
                }
                Err(_e) => {
                    if reference {
                        self.refpos = Some(refpos);
                    }
                    // End of file
                    return 0;
                }
            }
        }

        0
    }

    
    /// Sets the refpos values to be the current locations.  Prior to making a
    /// significant change to locations (legalization, for example), it may
    /// be useful to make a snapshot of original locations, so that the
    /// movement can be visualized.
    pub fn set_refpos(&mut self) {
        self.refpos = Some(self.cellpos.clone());
    }

    pub fn write_pl(&self, filepath: String, annotate: &Vec<String>) {
        let mut f = File::create(filepath).unwrap();
        writeln!(&mut f, "UCLA pl 1.0").unwrap();
        writeln!(&mut f, "# Generated by bookshelf_r.  HPWL {}", self.wl()).unwrap();
        for s in annotate {
            writeln!(&mut f, "# {}", s).unwrap();
        }
        for i in 0..self.cells.len() {
            let c = &self.cells[i];
            writeln!(
                &mut f,
                "{}  {} {}",
                c.name, self.cellpos[i].x, self.cellpos[i].y
            )
            .unwrap();
        }
    }

    /// Writes a PL formatted placement fil, marking all macro blocks as
    /// fixed.  Useful for setting fixed locations on macro blocks, so that
    /// the placement can be post-processed by an analytic placer.
    pub fn write_pl_fix(&self, filepath: String, annotate: &Vec<String>) {
        let mut f = File::create(filepath).unwrap();
        writeln!(&mut f, "UCLA pl 1.0").unwrap();
        writeln!(&mut f, "# Generated by bookshelf_r.  HPWL {}", self.wl()).unwrap();
        for s in annotate {
            writeln!(&mut f, "# {}", s).unwrap();
        }
        for i in 0..self.cells.len() {
            let c = &self.cells[i];
            if c.is_macro {
                writeln!(
                    &mut f,
                    "{}  {} {} : N /FIXED",
                    c.name, self.cellpos[i].x, self.cellpos[i].y
                )
                .unwrap();
            } else {
                writeln!(
                    &mut f,
                    "{}  {} {}",
                    c.name, self.cellpos[i].x, self.cellpos[i].y
                )
                .unwrap();
            }
        }
    }

    pub fn read_scl(&mut self, filepath: &Path) -> usize {
        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if LDBG {
            println!("First line of SCL file {}", line);
        }

        let mut num_rows = 0;
        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok(nr) = scan_fmt!(&line.to_lowercase(), "numrows : {d}", usize) {
            println!("SCL has {} rows", nr);
            num_rows = nr;
        } else {
            println!("Error on rows line {}", line)
        }

        for row in 0..num_rows {
            if LDBG {
                println!("Row {}", row);
            }
            // CoreRow Horizontal
            let line = BookshelfCircuit::getline(&mut reader).unwrap();

            let mut coordinate = 0 as f32;
            let mut height = 0 as f32;
            let mut sitewidth = 0 as f32;
            let mut sitespacing = 0 as f32;
            let mut orient = 0 as u32;
            let mut symmetry = 0 as u32;
            let mut origin = 0 as f32;
            let mut numsites = 0 as f32;

            // Coordinate : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(crd) = scan_fmt!(&line, " Coordinate : {d}", f32) {
                if LDBG {
                    println!("  Coord {}", crd);
                }
                coordinate = crd;
            }
            // Height : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(ht) = scan_fmt!(&line, " Height : {d}", f32) {
                if LDBG {
                    println!("  Height {}", ht);
                }
                height = ht;
            }
            // Sitewidth : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(sw) = scan_fmt!(&line, " Sitewidth : {d}", f32) {
                if LDBG {
                    println!("  Sitewidth {}", sw);
                }
                sitewidth = sw;
            }
            // Sitespacing : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(ss) = scan_fmt!(&line, " Sitespacing : {d}", f32) {
                if LDBG {
                    println!("  Spacing {}", ss);
                }
                sitespacing = ss;
            }
            // Siteorient : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(so) = scan_fmt!(&line, " Siteorient : {s}", String) {
                if LDBG {
                    println!("  Orient {}", so);
                }
                orient = 0;
            }
            // Sitesymmetry : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(sym) = scan_fmt!(&line, " Sitesymmetry : {s}", String) {
                if LDBG {
                    println!("  Symmetry {}", sym);
                }
                symmetry = 0;
            }
            // SubrowOrigin : n  Numsites : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok((sro, ns)) = scan_fmt!(
                &line.to_lowercase(),
                " subroworigin : {d} numsites : {d}",
                f32,
                f32
            ) {
                if LDBG {
                    println!("  SRO  {}  NS {}", sro, ns);
                }
                origin = sro;
                numsites = ns;
            }

            // End line
            let _line = BookshelfCircuit::getline(&mut reader).unwrap();
            let mut bounds = bbox::BBox::new();
            bounds.addpoint(origin, coordinate);
            bounds.addpoint(origin + numsites * sitewidth, coordinate + height);
            self.rows.push(Row {
                name: "row".to_string(),
                bounds: bounds,
                site_spacing: sitespacing,
            });
        }

        0
    }

    pub fn cell_area(&self) -> f32 {
        let mut tot_area = 0.0;
        for c in &self.cells {
            if c.terminal {
                // tot_pads = tot_pads + 1;
            } else {
                tot_area = tot_area + c.area();
            }
        }

        tot_area
    }

    pub fn summarize(&self) {
        println!("---- CIRCUIT SUMMARY INFORMATION ----");
        println!(
            "Circuit has {} cells, {} nets, {} rows",
            self.cells.len(),
            self.nets.len(),
            self.rows.len()
        );
        let mut tot_pads = 0;
        let mut tot_area = 0.0;
        for c in &self.cells {
            if c.terminal {
                tot_pads = tot_pads + 1;
            } else {
                tot_area = tot_area + c.area();
            }
        }

        let mut tot_row_area = 0.0;
        for r in &self.rows {
            tot_row_area = tot_row_area + r.bounds.area();
        }
        println!(
            "{} pads.\nTotal cell area: {}\nTotal row area: {}\nUtilization: {}",
            tot_pads,
            tot_area,
            tot_row_area,
            tot_area / tot_row_area
        );
        println!("Wire length: {}", self.wl());
        println!(
            "Cells: {} Macros: {} Terminals {}",
            self.num_cells, self.num_macros, self.num_terminals
        );
        println!("---------------");
        // for i in self.cells.len() - 10..self.cells.len() {
        //     println!(
        //         "Cell {} size {} x {}",
        //         self.cells[i].name, self.cells[i].w, self.cells[i].h
        //     );
        //     for p in &self.cells[i].pins {
        //         println!(
        //             "  Pin at {} {} net {}",
        //             p.dx, p.dy, self.nets[p.parent_net].name
        //         );
        //     }
        // }
        // for rn in 0..6.min(self.rows.len()) {
        //     println!("Row {}  ({}, {}) to ({}, {})", rn,
        //     self.rows[rn].bounds.llx, self.rows[rn].bounds.lly, self.rows[rn].bounds.urx, self.rows[rn].bounds.ury);
        // }
    }

    fn getline(reader: &mut BufReader<File>) -> std::io::Result<String> {
        loop {
            let mut line = String::new();
            let _len = reader.read_line(&mut line).unwrap();
            // println!("Read in {} bytes, line {}", _len, line);

            if _len == 0 {
                return std::result::Result::Err(Error::new(ErrorKind::Other, "end of file"));
            }

            if line.starts_with("#") {
                // println!("Skip comment.");
                continue;
            }

            if _len == 1 {
                continue;
            }

            return Ok(line.trim().to_string());
        }
        // Error::new(ErrorKind::Other, "Not reachable FILE IO error");
    }

    fn getmatch(reader: &mut BufReader<File>, fmt: String) -> std::io::Result<String> {
        loop {
            let line = match BookshelfCircuit::getline(reader) {
                Ok(line) => {
                    println!("Try to match {} and {}", line, fmt);
                    return Ok(line);
                }
                Err(e) => {
                    return Err(e);
                }
            };
        }
        // std::result::Result::Err(Error::new(ErrorKind::Other, "No match"))
    }

    pub fn cell_index(&self, name: &String) -> Option<usize> {
        let entry = self.cell_map.get(name);
        match entry {
            Some(rv) => return Some(*rv),
            _ => return None,
        }
    }

    fn find_cell(&mut self, newstr: String) -> usize {
        let v = self.cell_map.len();
        let entry = self.cell_map.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.cell_map.insert(newstr.clone(), v),
        };

        v
    }

    pub fn net_index(&self, name: &String) -> Option<usize> {
        let entry = self.net_map.get(name);
        match entry {
            Some(rv) => return Some(*rv),
            _ => return None,
        }
    }

    fn find_net(&mut self, newstr: String) -> usize {
        let v = self.net_map.len();
        let entry = self.net_map.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.net_map.insert(newstr.clone(), v),
        };

        v
    }
    fn find_macro(&mut self, newstr: String) -> usize {
        let v = self.macro_map.len();
        let entry = self.macro_map.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.macro_map.insert(newstr.clone(), v),
        };

        v
    }
    pub fn net_wl(&self, n: &Net) -> f32 {
        let mut first = true;
        let mut llx = 0.0;
        let mut lly = 0.0;
        let mut urx = 0.0;
        let mut ury = 0.0;
        for pref in &n.pins {
            let px =
                self.cellpos[pref.parent_cell].x + self.cells[pref.parent_cell].pins[pref.index].dx;
            let py =
                self.cellpos[pref.parent_cell].y + self.cells[pref.parent_cell].pins[pref.index].dy;

            if first {
                llx = px;
                urx = px;
                lly = py;
                ury = py;
                first = false;
            } else {
                llx = llx.min(px);
                urx = urx.max(px);
                lly = lly.min(py);
                ury = ury.max(py);
            }
        }
        let len = (urx - llx) + (ury - lly);

        len
    }

    pub fn wl(&self) -> f32 {
        let mut total = 0.0;
        let mut counter = 0;

        for n in &self.nets {
            if counter > 0 {
                println!("WL for net {}", n.name);
            }
            let mut first = true;
            let mut llx = 0.0;
            let mut lly = 0.0;
            let mut urx = 0.0;
            let mut ury = 0.0;
            for pref in &n.pins {
                let px = self.cellpos[pref.parent_cell].x
                    + self.cells[pref.parent_cell].pins[pref.index].dx;
                let py = self.cellpos[pref.parent_cell].y
                    + self.cells[pref.parent_cell].pins[pref.index].dy;
                if counter > 0 {
                    println!(
                        "Pinref cell {} pin {} at {} {}",
                        self.cells[pref.parent_cell].name, pref.index, px, py
                    );
                }
                if first {
                    llx = px;
                    urx = px;
                    lly = py;
                    ury = py;
                    first = false;
                } else {
                    llx = llx.min(px);
                    urx = urx.max(px);
                    lly = lly.min(py);
                    ury = ury.max(py);
                }
            }
            let len = (urx - llx) + (ury - lly);
            if counter > 0 {
                println!("BBox {} {}   {} {}   len {} ", llx, lly, urx, ury, len);
            }
            counter = counter - 1;
            total = total + len;
        }
        total as f32
    }
    pub fn core(&self) -> bbox::BBox {
        let mut result = bbox::BBox::new();
        if self.rows.len() > 1 {
            for r in &self.rows {
                result.expand(&r.bounds);
            }
        } else {
            // If no rows are specified, we create a square core area.
            let area = self.cell_area();
            let side = area.sqrt() * 1.10;
            result.addpoint(0.0, 0.0);
            result.addpoint(side, side);
        }
        result
    }
    pub fn mincore(&self) -> bbox::BBox {
        let mut core = self.core();
        let mut total = 0.0;
        for i in 0..self.cells.len() {
            if self.cells[i].terminal == false {
                total = total + self.cells[i].area();
            }
        }
        let core_area = core.area();
        let utilization = total / core_area;
        let scale = utilization.sqrt();

        let dx = core.dx() * scale;
        let dy = core.dy() * scale;

        let offset_x = (core.dx() - dx) * 0.5;
        let offset_y = (core.dy() - dy) * 0.5;

        let mut mincore = core;
        mincore.llx = mincore.llx + offset_x;
        mincore.lly = mincore.lly + offset_y;
        mincore.urx = mincore.llx + dx;
        mincore.ury = mincore.lly + dy;

        println!("Minimum core: {} --> {}", core, mincore);

        mincore
    }
    pub fn leftcore(&self) -> bbox::BBox {
        let mut core = self.core();
        let mut total = 0.0;
        for i in 0..self.cells.len() {
            if self.cells[i].terminal == false {
                total = total + self.cells[i].area();
            }
        }
        let core_area = core.area();
        let utilization = total / core_area;

        let dx = core.dx() * utilization;
        let dy = core.dy();

        let offset_x = 0.0;
        let offset_y = 0.0;

        let mut leftcore = core;
        leftcore.llx = leftcore.llx + offset_x;
        leftcore.lly = leftcore.lly + offset_y;
        leftcore.urx = leftcore.llx + dx;
        leftcore.ury = leftcore.lly + dy;

        println!("Left-aligned core: {} --> {}", core, leftcore);

        leftcore
    }
    pub fn pinloc(&self, pr: &PinRef) -> (f32, f32) {
        let px = self.cellpos[pr.parent_cell].x + self.cells[pr.parent_cell].pins[pr.index].dx;
        let py = self.cellpos[pr.parent_cell].y + self.cells[pr.parent_cell].pins[pr.index].dy;
        (px, py)
    }

    pub fn read_blockpacking(filename: String) -> BookshelfCircuit {
        let f = File::open(filename.clone()).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();

        if LDBG {
            println!("Returned line {}", line);
        }

        let parsed = sscanf::sscanf!(line, "BlockPacking : {str} {str} {str}");
        let (blockf, netf, plf) = parsed.unwrap();

        println!("Block file {}", blockf);

        let path = Path::new(&filename);

        let mut bc = BookshelfCircuit::new();
        bc.read_blocknodes(path.with_file_name(blockf).as_path());
        bc.read_nets(path.with_file_name(netf).as_path());
        bc.read_pl(path.with_file_name(plf).as_path(), false);
        bc.unit_x = 1.0;
        bc.unit_y = 1.0;

        bc
    }

    pub fn read_blocknodes(&mut self, filepath: &Path) {
        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let mut numsoft = 0;
        let mut numhard = 0;
        let mut numterm = 0;

        loop {
            let line = BookshelfCircuit::getline(&mut reader);
            match line {
                Ok(l) => {
                    if LDBG {
                        println!("Read nodes line {}", l);
                    }
                    if let Ok(ns) = scan_fmt!(&l, "NumSoftRectangularBlocks : {}", u32) {
                        println!("Got {} soft blocks", ns);
                        numsoft = ns;
                    }
                    if let Ok((bname, corners, x0, y0, x1, y1, x2, y2, x3, y3)) = scan_fmt!(
                        &l,
                        "{} hardrectilinear {} ({}, {}) ({}, {}) ({}, {}) ({}, {})",
                        String,
                        u32,
                        f32,
                        f32,
                        f32,
                        f32,
                        f32,
                        f32,
                        f32,
                        f32
                    ) {
                        if LDBG {
                            println!("Got hard macro {}", bname);
                        }
                        let cn = self.find_cell(bname.clone());

                        let w = x2 - x0;
                        let h = y2 - y0;
                        let c = Cell {
                            name: bname,
                            w: w,
                            h: h,
                            pins: Vec::new(),
                            terminal: false,
                            soft: None,
                            is_macro: false,
                            can_rotate: false,
                        };
                        self.cells.push(c);
                        let cp = point::Point { x: 0.0, y: 0.0 };
                        self.cellpos.push(cp);
                        self.orient.push(Orientation { orient: 0 });
                    }
                    if let Ok(tname) = scan_fmt!(&l, "{} terminal", String) {
                        if LDBG {
                            println!("Got terminal {}", tname);
                        }
                        let cn = self.find_cell(tname.clone());

                        self.cells.push(Cell {
                            name: tname,
                            w: 1.0,
                            h: 1.0,
                            pins: Vec::new(),
                            terminal: true,
                            soft: None,
                            is_macro: false,
                            can_rotate: true,
                        });
                        self.cellpos.push(point::Point { x: 0.0, y: 0.0 });
                        self.orient.push(Orientation { orient: 0 });
                    }
                }
                _ => {
                    return;
                }
            }
        }
    }
}

use crate::marklist;
use crate::marklist::MarkList;
use std::os::raw::{c_int, c_uint, c_ulong};

pub struct HyperParams {
    pub cellmark: marklist::MarkList,
    pub netmark: marklist::MarkList,
    pub horizontal: bool,
    pub split_point: f32,
    pub partition: Vec<c_int>,
    pub bias: f32,
    pub k: u32,
    pub term_prop: bool,
    pub passes: usize,
    pub seed: usize,
    pub edgeweight: usize, // Different edge weighting modes
}

impl HyperParams {
    pub fn new(ckt: &crate::bookshelf::BookshelfCircuit) -> HyperParams {
        HyperParams {
            cellmark: marklist::MarkList::new(ckt.cells.len()),
            netmark: marklist::MarkList::new(ckt.nets.len()),
            horizontal: false,
            split_point: 0.0,
            partition: vec![-1; ckt.cells.len()],
            bias: 0.5,
            k: 2,
            term_prop: true,
            passes: 1,
            seed: 8675309,
            edgeweight: 0,
        }
    }
}

// pub struct HyperGraph {
//     pub vtxwt: Vec<c_int>,
//     pub hewt: Vec<c_int>,
//     pub part: Vec<c_int>,
//     pub eind: Vec<c_ulong>,
//     pub eptr: Vec<c_uint>,
// }
// use metapartition::hypergraph::HyperGraph;
use hypergraph::HyperGraph;

pub fn hypergraph(
    ckt: &BookshelfCircuit,
    cells: &Vec<usize>,
    params: &mut HyperParams,
) -> HyperGraph {
    HyperGraph {
        vtxwt: Vec::new(),
        hewt: Vec::new(),
        part: Vec::new(),
        eind: Vec::new(),
        eptr: Vec::new(),
    }
}

impl BookshelfCircuit {
    // pub fn new() -> HyperGraph {
    //     HyperGraph {
    //         vtxwt: Vec::new(),
    //         hewt: Vec::new(),
    //         part: Vec::new(),
    //         eind: Vec::new(),
    //         eptr: Vec::new()
    //     }
    // }
    pub fn build_graph(&self, cells: &Vec<usize>, params: &mut HyperParams) -> HyperGraph {
        params.cellmark.clear();
        params.netmark.clear();
        // println!("Split {} cells, total weight {}", cells.len(), ckt.cellweights(cells));
        let mut warned = false;

        let mut hg = HyperGraph::new();

        for i in 0..cells.len() {
            if params.cellmark.marked[cells[i]] {
                println!("Cell already marked!");
            }
            params.cellmark.mark(cells[i]);
            let cell_id = params.cellmark.list[i];
            // Mark all connected nets

            let cell = &self.cells[cell_id];
            // println!("Marking cell {} with {} pins", cell.name, cell.pins.len());
            for pininstance in &cell.pins {
                let net_id = pininstance.parent_net;
                params.netmark.mark(net_id);
            }
        }
        // println!("Marked {} nets", params.netmark.list.len());
        let mut cardinality = vec![0; params.netmark.list.len()];
        let mut sinks = vec![false; params.netmark.list.len()];
        let mut sources = vec![false; params.netmark.list.len()];

        // let mut propagated = 0;

        for net_id in &params.netmark.list {
            // println!(" Marked net {} {}", nidx, ckt.nets[*nidx].name);
            for pr in &self.nets[*net_id].pins {
                // println!("  Ref cell {} marked: {}", pr.parentCell, cellmark.marked[pr.parentCell]);
                if params.cellmark.marked[pr.parent_cell] {
                    cardinality[params.netmark.index[*net_id]] =
                        cardinality[params.netmark.index[*net_id]] + 1;
                } else {
                    if params.term_prop {
                        let (px, py) = self.pinloc(pr);
                        // Check with horizontal, vertical, set the sinks
                        let split_value;
                        if params.horizontal {
                            split_value = py;
                        } else {
                            split_value = px;
                        }

                        if split_value < params.split_point {
                            sources[params.netmark.index[*net_id]] = true;
                        } else {
                            sinks[params.netmark.index[*net_id]] = true;
                        }
                    }
                }
            }
            // Add to propagated if we need to
            if cardinality[params.netmark.index[*net_id]] == 0 {
                if !warned {
                    println!("Net {} was marked, but not detected", net_id);
                    warned = true;

                    for cell_id in 0..cells.len() {
                        for pininstance in &self.cells[cell_id].pins {
                            if *net_id == pininstance.parent_net {
                                println!(
                                    "FOUND cell {} mark {}",
                                    cell_id, params.cellmark.marked[cell_id]
                                );
                            }
                        }
                    }
                }
            }
        }
        //let mut vtxwt: Vec<c_int> = Vec::new();
        //let mut hewt: Vec<c_int> = Vec::new();
        //let mut part: Vec<c_int> = Vec::new();
        hg.vtxwt.clear();
        hg.hewt.clear();
        hg.part.clear();

        // Put the cells onto the list
        let mut tot_area = 0.0;
        for cell_id in &params.cellmark.list {
            let cell = &self.cells[*cell_id];
            hg.vtxwt.push(cell.area() as c_int);
            tot_area = tot_area + cell.area();
            // vtxwt.push(1 as c_int);
            // println!("Added cell  {} index {} h {} w {}", cell.name, *c, cell.h, cell.w);
            hg.part.push(-1);
        }

        // Fixed source and sink vertex IDs
        // These are only used if terminal propagation is enabled
        let mut src_id = 0;
        let mut sink_id = 0;
        if params.term_prop {
            src_id = hg.vtxwt.len();
            hg.vtxwt.push(1);
            hg.part.push(0);
            sink_id = hg.vtxwt.len();
            hg.vtxwt.push(1);
            hg.part.push(1);
        }

        // println!("Partitioner total vtxw area: {} with {} marked cells", tot_area, params.cellmark.list.len());

        // Now go through the nets -- and we'll add sinks and sources as we
        // go along as needed
        // let mut eind: Vec<c_ulong> = Vec::new();
        hg.eind.clear();
        let mut totfix = 0;
        hg.eind.push(0 as c_ulong);
        // let mut eptr: Vec<c_uint> = Vec::new();
        let mut tot_prop = 0;
        for net_id in &params.netmark.list {
            let mut card = 0;

            for pr in &self.nets[*net_id].pins {
                if params.cellmark.marked[pr.parent_cell] {
                    hg.eptr
                        .push(params.cellmark.index[pr.parent_cell] as c_uint);
                    card = card + 1;
                }
            }
            // let mut wt = 10;
            // if sources[params.netmark.index[*net_id]] || sinks[params.netmark.index[*net_id]] {
            //     wt = 2;
            // }
            // if card < 3 {
            //     wt = wt + 1;
            // }
            match params.edgeweight {
                0 => {
                    hg.hewt.push(1 as c_int);
                }
                1 => {
                    if sources[params.netmark.index[*net_id]]
                        || sinks[params.netmark.index[*net_id]]
                    {
                        hg.hewt.push(6 as c_int);
                    } else {
                        hg.hewt.push(5 as c_int);
                    }
                }
                _ => {
                    hg.hewt.push(1 as c_int);
                }
            }

            // Maybe push the source and sink -- and add vertex weights of zero
            // for these, and make them partition location -1
            if params.term_prop {
                if sources[params.netmark.index[*net_id]] {
                    // hg.eptr.push(hg.vtxwt.len() as u32);
                    hg.eptr.push(src_id as u32);
                    //hg.part.push(0);
                    //hg.vtxwt.push(1);
                    totfix = totfix + 1;
                    card = card + 1;
                    tot_prop = tot_prop + 1;
                }
                if sinks[params.netmark.index[*net_id]] {
                    //hg.eptr.push(hg.vtxwt.len() as u32);
                    hg.eptr.push(sink_id as u32);
                    //hg.part.push(1);
                    //hg.vtxwt.push(1);
                    totfix = totfix + 1;
                    card = card + 1;
                    tot_prop = tot_prop + 1;
                }
            }

            // Now push the eptr that ends this (it'll be the eptr for the next net)
            hg.eind.push(hg.eptr.len() as c_ulong);
            if card < 2 && params.term_prop && !warned {
                println!(
                    "Cardinality {} net {} source {} sink {}",
                    card,
                    params.netmark.index[*net_id],
                    sources[params.netmark.index[*net_id]],
                    sinks[params.netmark.index[*net_id]]
                );
                warned = true;
            }
        }
        // println!("{} propagated terminals", tot_prop);
        hg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
