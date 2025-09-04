### 20241004

Got PL file reading working. That was easy.  Now to
compute HPWL, and see if it matches what I think it
should match...

### 20241107

Added in SCL file reading, so that I've got the rows.
Moved the XY location out of the cell structure, will
use cellpos (or maybe convert to location?).  Perhaps
have a separate orientation vector?  Yeah, probably
a good idea.

Computing HPWL correctly, so it's all loaded in OK.
May be time to start working on fsdc_r.

### 20250411

Fixing up the hypergraph creator, so that it doesn't
add in extra terminals if terminal propagation is
turned off.

### 20250425

Adding in some more fields, so that I can handle
cell orientation, flips, and so on.  Added a refpos
structure, reference locations.  Should switch from
using the pstools location to bookshelf location,
avoids a potential dependency.  Will add some display
built-ins to generate printable text for cells, pins,
nets, and so on.  There's a bunch of utility functions
that would be nice to have, let's get them in today....

On second thought -- I'll keep bbox and point from
PStools.  Library is being brought in to do circuit
renders, so we'll have those, and convenient to have
all the code on the same platform.

But... might wind up with 3D points?  And 3D
bounding boxes?  Hmmmm.

### 20250902

Finally adding in cell orientation.  Annoying, man,
totally annoying.