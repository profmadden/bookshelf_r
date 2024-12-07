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

const LDBG: bool = false;

// mod crate::bbox;
// mod point;
use pstools_r::bbox;
use pstools_r::point;

// use crate::point;


use std::fmt;
use pstools_r;


// PinInstances are in the vector for the cells
pub struct PinInstance {
    pub name: String,
    pub dx: f32,
    pub dy: f32,
    pub parent_cell: usize,
    pub parent_net: usize,
}

// PinRefs are in the vector for the nets
pub struct PinRef {
    pub parent_cell: usize,
    pub index: usize,
}


pub struct Orientation {
    pub orient: u8,
}

pub struct Cell {
    pub name: String,
    pub w: f32,
    pub h: f32,
    // pub x: f32,
    // pub y: f32,
    pub pins: Vec<PinInstance>,
    pub terminal: bool,
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
pub struct BookshelfCircuit {
    pub counter: i32,
    pub cells: Vec<Cell>,
    pub cellpos: Vec<point::Point>,
    pub orient: Vec<Orientation>,
    pub nets: Vec<Net>,
    pub macros: Vec<Macro>,
    pub rows: Vec<Row>,
    pub cell_map: HashMap<String, usize>,
    pub net_map: HashMap<String, usize>,
    pub macro_map: HashMap<String, usize>,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

impl fmt::Display for BookshelfCircuit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name {} cells, {} nets, core {}, HPWL {}", self.cells.len(), self.nets.len(), self.core(), self.wl())
    }
}

impl BookshelfCircuit {
    pub fn new() -> BookshelfCircuit {
        let bc = BookshelfCircuit {
            counter: 0,
            cells: Vec::new(),
            cellpos: Vec::new(),
            orient: Vec::new(),
            nets: Vec::new(),
            macros: Vec::new(),
            rows: Vec::new(),
            cell_map: HashMap::new(),
            net_map: HashMap::new(),
            macro_map: HashMap::new(),
        };

        bc
    }
    pub fn postscript(&self, filename: String) {
        let mut pst = pstools_r::PSTool::new();
        pst.set_color(0.3, 0.4, 0.2, 1.0);
        for i in 0..self.cells.len() {
            pst.add_box(self.cellpos[i].x, self.cellpos[i].y,
            self.cellpos[i].x + self.cells[i].w - 0.5,
        self.cellpos[i].y + self.cells[i].h - 0.5);
        }
        
        pst.generate(filename);
    }

    pub fn cellweights(&self, cells:&Vec<usize>) -> f32 {
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

    pub fn read_aux(filename: String) -> BookshelfCircuit {
        let f = File::open(filename.clone()).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();

        if LDBG {
          println!("Returned line {}", line);
        }

        let parsed = sscanf::sscanf!(line, "RowBasedPlacement : {str} {str} {str} {str} {str}");
        let (nodef, netf, wtf, plf, sclf) = parsed.unwrap();

        println!("Node file {}", nodef);

        let path = Path::new(&filename);

        let mut bc = BookshelfCircuit::new();

        bc.read_nodes(path.with_file_name(nodef).as_path());
        bc.read_nets(path.with_file_name(netf).as_path());
        bc.read_pl(path.with_file_name(plf).as_path());
        bc.read_scl(path.with_file_name(sclf).as_path());


        if LDBG {
          println!("BC counter is {}", bc.counter);
        }

        bc
    }

    pub fn read_nodes(&mut self, filepath: &Path) -> usize {
        // println!("Opening {}", filename);

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        println!("First line of nodes file {}", line);

        self.counter = self.counter + 1;

        // Look for the nodes line
        let mut num_node = 0 as i32;
        let mut num_term = 0 as i32;

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nn)) = scan_fmt!(&line, "NumNodes : {d}", i32) {
            if LDBG { println!("Scan fmt worked, value is {}", nn);}
            num_node = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nt)) = scan_fmt!(&line, "NumTerminals : {d}", i32) {
            if LDBG { println!("Scan fmt worked, value is {}", nt);}
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
                };

                self.cells.push(c);

                let cp = point::Point {
                    x: 0.0,
                    y: 0.0,
                    // orientation: 0,
                };
                self.cellpos.push(cp);

                let co = Orientation {
                    orient: 0
                };
                self.orient.push(co);

            } else {
                println!("Not ok match");
            }
        }

        0
    }

    pub fn read_nets(&mut self, filepath: &Path) -> usize {
        // println!("Opening {}", filename);

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        println!("First line of nets file {}", line);

        self.counter = self.counter + 1;

        // Look for the nodes line
        let mut num_nets = 0 as usize;
        let mut num_pins = 0 as usize;

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nn)) = scan_fmt!(&line, "NumNets : {d}", usize) {
            if LDBG { println!("Scan fmt worked, value is {}", nn);}
            num_nets = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok(np) = scan_fmt!(&line, "NumPins : {d}", usize) {
            if LDBG { println!("Scan fmt worked, value is {}", np);}
            num_pins = np;
        }

        println!("Nets file has {} nets, {} pins", num_nets, num_pins);
        self.nets = Vec::with_capacity(num_nets);

        for nidx in 0..num_nets {
            let line = BookshelfCircuit::getline(&mut reader).unwrap();

            if let Ok((nd, nn)) = scan_fmt!(&line, "NetDegree : {d} {}", usize, String) {
                if LDBG { println!("Net {} degree {}", nn, nd);}
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
                    if LDBG { println!("Pin line {}", line);}

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
                        if LDBG { println!("PIN NAME {} sdx {} sdy {}", cellname, sdx, sdy);}

                        dx = sdx.parse().unwrap();
                        dy = sdy.parse().unwrap();
                    } else if let Ok((cn, sdir)) = scan_fmt!(&line, " {} {}", String, String) {
                        cellname = cn.clone();
                    }

                    if LDBG {println!("Create pin for cell {} at {} {}", cellname, dx, dy);}
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
                    };
                    self.cells[cidx].pins.push(pi);
                }
                self.nets.push(net);
            }
        }

        0
    }

    pub fn read_pl(&mut self, filepath: &Path) -> usize {
        // println!("Opening {}", filename);

        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if LDBG { println!("First line of PL file {}", line);}

        loop {
            let line = BookshelfCircuit::getline(&mut reader);
            match line {
                Ok(l) => {
                    if LDBG { println!("Read PL line {}", l);}
                    if let Ok((cellname, x, y)) = scan_fmt!(&l, " {} {} {}", String, String, String) {
                        let cidx = self.find_cell(cellname);
                        self.cellpos[cidx].x = x.parse().unwrap();
                        self.cellpos[cidx].y = y.parse().unwrap();
                        if LDBG{ println!("  Locate cell at {} {}", self.cellpos[cidx].x, self.cellpos[cidx].y);}
                    }
                    
                },
                Err(_e) => {
                    // End of file
                    return 0
                }
            }
        }
        0
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
            writeln!(&mut f, "{}  {} {}", c.name, self.cellpos[i].x, self.cellpos[i].y).unwrap();
        }
    }

    pub fn read_scl(&mut self, filepath: &Path) -> usize {
        let f = File::open(filepath).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if LDBG { println!("First line of SCL file {}", line);}

        let mut num_rows = 0;
        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok(nr) = scan_fmt!(&line, "Numrows : {d}", usize) {
            println!("SCL has {} rows", nr);
            num_rows = nr;
        }

        for row in 0..num_rows {

            if LDBG {println!("Row {}", row);}
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
                if LDBG {println!("  Coord {}", crd);}
                coordinate = crd;
            }
            // Height : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(ht) = scan_fmt!(&line, " Height : {d}", f32) {
              if LDBG {println!("  Height {}", ht);}
              height = ht;
            }
            // Sitewidth : n 
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(sw) = scan_fmt!(&line, " Sitewidth : {d}", f32) {
                if LDBG {println!("  Sitewidth {}", sw);}
                sitewidth = sw;
            }
            // Sitespacing : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(ss) = scan_fmt!(&line, " Sitespacing : {d}", f32) {
              if LDBG {println!("  Spacing {}", ss);}
              sitespacing = ss;
            }
            // Siteorient : n 
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(so) = scan_fmt!(&line, " Siteorient : {s}", String) {
                if LDBG {println!("  Orient {}", so);}
                orient = 0;
            }
            // Sitesymmetry : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok(sym) = scan_fmt!(&line, " Sitesymmetry : {s}", String) {
              if LDBG {println!("  Symmetry {}", sym);}
              symmetry = 0;
            }
            // SubrowOrigin : n  Numsites : n
            let line = BookshelfCircuit::getline(&mut reader).unwrap();
            if let Ok((sro, ns)) = scan_fmt!(&line, " SubrowOrigin : {d} Numsites : {d}", f32, f32) {
              if LDBG {println!("  SRO  {}  NS {}", sro, ns);}
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

    pub fn summarize(&self) {
        println!("---- CIRCUIT SUMMARY INFORMATION ----");
        println!("Circuit has {} cells, {} nets, {} rows", self.cells.len(), self.cells.len(), self.rows.len());
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
        println!("{} pads.\nTotal cell area: {}\nTotal row area: {}\nUtilization: {}", tot_pads, tot_area, tot_row_area, tot_area/tot_row_area);
        println!("Wire length: {}", self.wl());
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

    fn find_cell(&mut self, newstr: String) -> usize {
        let v = self.cell_map.len();
        let entry = self.cell_map.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.cell_map.insert(newstr.clone(), v),
        };

        v
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
                let px = self.cellpos[pref.parent_cell].x + self.cells[pref.parent_cell].pins[pref.index].dx;
                let py = self.cellpos[pref.parent_cell].y + self.cells[pref.parent_cell].pins[pref.index].dy;
                if counter > 0 {
                    println!("Pinref cell {} pin {} at {} {}", self.cells[pref.parent_cell].name, pref.index, px, py);
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
        for r in &self.rows {
            result.expand(&r.bounds);
        }
        result
    }
    pub fn pinloc(&self, pr: &PinRef) -> (f32, f32) {
        let px = self.cellpos[pr.parent_cell].x + self.cells[pr.parent_cell].pins[pr.index].dx;
        let py = self.cellpos[pr.parent_cell].y + self.cells[pr.parent_cell].pins[pr.index].dy;
        (px, py)
    }
   
  
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}


// DATABASE EXAMPLE
// http://jakegoulding.com/rust-ffi-omnibus/objects/

pub struct ZipCodeDatabase {
    population: HashMap<String, u32>,
}

impl ZipCodeDatabase {
    fn new() -> ZipCodeDatabase {
        ZipCodeDatabase {
            population: HashMap::new(),
        }
    }

    fn populate(&mut self) {
        for i in 0..100_000 {
            let zip = format!("{:05}", i);
            self.population.insert(zip, i);
        }
    }

    fn population_of(&self, zip: &str) -> u32 {
        self.population.get(zip).cloned().unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn zip_code_database_new() -> *mut ZipCodeDatabase {
    Box::into_raw(Box::new(ZipCodeDatabase::new()))
}

#[no_mangle]
pub extern "C" fn zip_code_database_free(ptr: *mut ZipCodeDatabase) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn zip_code_database_populate(ptr: *mut ZipCodeDatabase) {
    let database = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    database.populate();
}

#[no_mangle]
pub extern "C" fn zip_code_database_population_of(
    ptr: *const ZipCodeDatabase,
    zip: *const c_char,
) -> u32 {
    let database = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };
    let zip = unsafe {
        assert!(!zip.is_null());
        CStr::from_ptr(zip)
    };
    let zip_str = zip.to_str().unwrap();
    database.population_of(zip_str)
}
