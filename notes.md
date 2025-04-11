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
