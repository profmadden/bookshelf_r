# bookshelf_r
UCLA Bookshelf interface in Rust

20241004 Initial setup.  Looks like I'm able to read in files, and build the
basic data structures without too much grief.

Run the standalone with
<pre>
cargo run -- -a input/ibm01.aux
</pre>

The -a is for the Bookshelf-format AUX file.  You can also use --aux auxfile_name.

For block placking based designs, use the switch -b.

<pre>
cargo run -- -a input/n100.aux -b
</pre>

The example program will generate a PostScript layout of the circuits.  This can be
converted to PDF (a suggested GhostScript command line is in the first part of the
ps file -- look for "gs -o ????.pdf -sDEVICE=pdfwrite -dEPSCrop ????.ps").


