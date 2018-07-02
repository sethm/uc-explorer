Symbolics Microcode Explorer
============================

This project contains a program to analyze Symbolics microcode files.

History
-------

Version 0.1.0

Last Updated: 02-July-2018

Usage
-----

Symbolics Microcode Explorer is an interactive, command-driven program.

To start the program:

    uc-explorer [-f <ucode_file>]

From there, you are presented with a prompt:

    uc-explorer>

Valid commands are:

  - **help**: Get a short help summary
  - **show**: Show summary info about the microcode file
  - **load &lt;filename&gt;**: Load a Microcode file
  - **dump &lt;filename&gt;**: Disassemble and dump to a file
  - **quit**: Quit the program

WARNING: The disassembly process produces around 4.5MB of output!

TODO
----

Plans on the horizon are:

1. Full exploration of AMEM, BMEM, and CMEM using keystrokes
   to navigate between words.
2. Better disassembly format.

License
-------

Copyright 2017, Seth J. Morabito &lt;web@loomcom.com&gt;

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see &lt;https://www.gnu.org/licenses/&gt;.
