// Suppose we have a set of items (indexed by a usize).  Cells,
// nets, and so on.  We want to be able to mark a subset of these
// as relevant, be able to tell if something has been marked, and if
// it has been marked, map it to range of 0..number_marked
pub struct MarkList {
    pub len: usize,  // Number of elements, 0 to len-1
    pub marked: Vec<bool>, // Marker that an item has been marked
    pub list: Vec<usize>, // List of actively marked items
    pub index: Vec<usize>, // Index number of an item
}

impl MarkList {
    pub fn new(len: usize) -> MarkList{
        MarkList {
            len: len,
            marked: vec![false; len],
            list: Vec::new(),
            index: vec![0; len],
        }
    }
    pub fn mark(&mut self, n: usize) {
        if !self.marked[n] {
            self.marked[n] = true;
            self.index[n] = self.list.len();
            self.list.push(n);
        }
    }
    pub fn clear(&mut self) {
        for v in &self.list {
            self.marked[*v] = false;
        }
        self.list.clear();
    }
    pub fn dump(&self) {
        println!("There are {} elements marked", self.list.len());
        for v in &self.list {
            println!("Element {} index {}", *v, self.index[*v]);
        }
    }
}