# bookshelf_r
UCLA Bookshelf interface in Rust


Run the standalone with
<pre>
cargo run -- -a input/ibm01.aux -P example.ps 
</pre>

The -a is for the Bookshelf-format AUX file.  You can also use --aux auxfile_name.

The -P generates a PostScript file output.

For block placking based designs, use the switch -b.

<pre>
cargo run -- -a input/n100.aux -b -P example.ps
</pre>

The example program will generate a PostScript layout of the circuits.  This can be
converted to PDF (a suggested GhostScript command line is in the first part of the
ps file -- look for "gs -o ????.pdf -sDEVICE=pdfwrite -dEPSCrop ????.ps").

<pre>
gs -o example.pdf -sDEVICE=pdfwrite -dEPSCrop example.ps
</pre>

This library is included as part of a variety of other physical design tools.  It reads GSRC Bookshelf format files, and creates a Rust BookshelfCircuit object that contains cells, nets, rows, and so on.
