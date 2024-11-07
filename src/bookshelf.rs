extern crate libc;
use libc::c_char;
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


pub fn callme() {
    println!("Call me");
}

// PinInstances are in the vector for the cells
pub struct PinInstance {
    pub name: String,
    pub dx: f32,
    pub dy: f32,
    pub parentCell: usize,
    pub parentNet: usize,
}

// PinRefs are in the vector for the nets
pub struct PinRef {
    pub parentCell: usize,
    pub index: usize,
}

pub struct Cell {
    pub name: String,
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
    pub pins: Vec<PinInstance>,
    pub terminal: bool,
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
    pub llx: f32,
    pub lly: f32,
    pub urx: f32,
    pub ury: f32,
    pub siteSpacing: f32,
}

pub struct BookshelfCircuit {
    pub counter: i32,
    pub cells: Vec<Cell>,
    pub nets: Vec<Net>,
    pub macros: Vec<Macro>,
    pub rows: Vec<Row>,
    pub cellMap: HashMap<String, usize>,
    pub netMap: HashMap<String, usize>,
    pub macroMap: HashMap<String, usize>,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

impl BookshelfCircuit {
    pub fn new() -> BookshelfCircuit {
        let mut bc = BookshelfCircuit {
            counter: 0,
            cells: Vec::new(),
            nets: Vec::new(),
            macros: Vec::new(),
            rows: Vec::new(),
            cellMap: HashMap::new(),
            netMap: HashMap::new(),
            macroMap: HashMap::new(),
        };

        bc
    }

    pub fn read_aux(filename: String) -> BookshelfCircuit {
        let f = File::open(filename.clone()).unwrap();
        let mut reader = BufReader::with_capacity(32000, f);
        let line = BookshelfCircuit::getline(&mut reader).unwrap();

        println!("Returned line {}", line);

        let parsed = sscanf::sscanf!(line, "RowBasedPlacement : {str} {str} {str} {str} {str}");
        let (nodef, netf, wtf, plf, sclf) = parsed.unwrap();

        println!("Node file {}", nodef);

        let path = Path::new(&filename);

        let mut bc = BookshelfCircuit::new();

        bc.read_nodes(path.with_file_name(nodef).as_path());
        bc.read_nets(path.with_file_name(netf).as_path());
        bc.read_pl(path.with_file_name(plf).as_path());

        println!("BC counter is {}", bc.counter);

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
            println!("Scan fmt worked, value is {}", nn);
            num_node = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((nt)) = scan_fmt!(&line, "NumTerminals : {d}", i32) {
            println!("Scan fmt worked, value is {}", nt);
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
                    x: 0.0,
                    y: 0.0,
                    pins: Vec::new(),
                    terminal: isterminal,
                };

                self.cells.push(c);
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
            println!("Scan fmt worked, value is {}", nn);
            num_nets = nn;
        }

        let line = BookshelfCircuit::getline(&mut reader).unwrap();
        if let Ok((np)) = scan_fmt!(&line, "NumPins : {d}", usize) {
            println!("Scan fmt worked, value is {}", np);
            num_pins = np;
        }

        println!("Nets file has {} nets, {} pins", num_nets, num_pins);
        self.nets = Vec::with_capacity(num_nets);

        for nidx in 0..num_nets {
            let line = BookshelfCircuit::getline(&mut reader).unwrap();

            if let Ok((nd, nn)) = scan_fmt!(&line, "NetDegree : {d} {}", usize, String) {
                println!("Net {} degree {}", nn, nd);
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
                    println!("Pin line {}", line);

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
                        println!("PIN NAME {} sdx {} sdy {}", cellname, sdx, sdy);

                        dx = sdx.parse().unwrap();
                        dy = sdy.parse().unwrap();
                    } else if let Ok((cn, sdir)) = scan_fmt!(&line, " {} {}", String, String) {
                        cellname = cn.clone();
                    }

                    println!("Create pin for cell {} at {} {}", cellname, dx, dy);
                    let cidx = self.find_cell(cellname);

                    let pr = PinRef {
                        parentCell: cidx,
                        index: self.cells[cidx].pins.len(),
                    };
                    net.pins.push(pr);
                    let pi = PinInstance {
                        name: "".to_string(),
                        dx: dx,
                        dy: dy,
                        parentCell: cidx,
                        parentNet: nidx,
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
        println!("First line of PL file {}", line);

        loop {
            let line = BookshelfCircuit::getline(&mut reader);
            match line {
                Ok(l) => {
                    println!("Read PL line {}", l);
                    if let Ok((cellname, x, y)) = scan_fmt!(&l, " {} {} {}", String, String, String) {
                        let cidx = self.find_cell(cellname);
                        self.cells[cidx].x = x.parse().unwrap();
                        self.cells[cidx].y = y.parse().unwrap();
                        println!("  Locate cell at {} {}", self.cells[cidx].x, self.cells[cidx].y);
                    }
                    
                },
                Err(e) => {
                    return 0
                }
            }
        }
        0
    }

    pub fn summarize(&self) {
        println!("Circuit has {} cells", self.cells.len());
        for i in self.cells.len() - 10..self.cells.len() {
            println!(
                "Cell {} size {} x {}",
                self.cells[i].name, self.cells[i].w, self.cells[i].h
            );
            for p in &self.cells[i].pins {
                println!(
                    "  Pin at {} {} net {}",
                    p.dx, p.dy, self.nets[p.parentNet].name
                );
            }
        }
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
        Error::new(ErrorKind::Other, "Not reachable FILE IO error");
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
        std::result::Result::Err(Error::new(ErrorKind::Other, "No match"))
    }

    fn find_cell(&mut self, newstr: String) -> usize {
        let v = self.cellMap.len();
        let entry = self.cellMap.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.cellMap.insert(newstr.clone(), v),
        };

        v
    }
    fn find_net(&mut self, newstr: String) -> usize {
        let v = self.netMap.len();
        let entry = self.netMap.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.netMap.insert(newstr.clone(), v),
        };

        v
    }
    fn find_macro(&mut self, newstr: String) -> usize {
        let v = self.macroMap.len();
        let entry = self.macroMap.get(&newstr);
        match entry {
            Some(rv) => return *rv,
            None => self.macroMap.insert(newstr.clone(), v),
        };

        v
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