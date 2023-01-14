# SVG Path Simplifier

The `svgps` program converts `.svg` images into MoveTo/LineTo/CubicBezierTo/ClosePath commands
contained in a [`.svgcom`](#svgcom-format) file.

The initial goal was to help plotters understand SVG in the same way that people do.

It utilizes the [usvg](https://github.com/RazrFalcon/resvg/tree/master/usvg) crate for SVG parsing and simplification.

## Features

* Converting `.svg` to `.svgcom`
* Converting `.svgcom` to `.svg` for previewing
* Selecting only stroked paths for the conversion
* Autocutting path segments that are not visible because of being covered by other figures
  (think of this as of a depth test)
* Polishing the result (removing the paths that are too short)

### Usage

```sh
svgps help generate # Try this first to see all the options
svgps generate INPUT.svg OUTPUT.svgcom [OPTIONS...]
svgps render INPUT.svgcom OUTPUT.svg [OPTIONS...]
```

### SvgCom format

SvgCom is a text format for vector graphics outline representation that borrows a lot from the
[`<path:d>`](https://www.w3.org/TR/SVG/paths.html#TheDProperty) format of SVG. 

The file extension is `.svgcom`.

The file content is of the form:

```
<viewbox_width: uint32>
<SPACE>
<viewbox_height: uint32>
<SPACE>
<number_of_commands: uint32>
<SPACE>
<number_of_coordinates: uint32>
<EOL>

<command: "M" | "L" | "C" | "Z">... (not delimited)
<EOL>

<coordinate: float64>... (delimited by <SPACE>)

[<EOL> <garbage: text>]
```

where `<EOL>` is `<LF>`, not `<CR><LF>`, and `<SPACE>` is a *single* ASCII charater 32.

Additional ("garbage") lines in the end of the file are optional and are never read by the converter.

The coordinate list specifies 2D point coordinates, thus the number of coordinates must be even.

<!-- The `Z` (ClosePath) command in SvgCom, unlike in SVG, does not draw a line.
It only preserves the fact that a path segment is closed.
Thus, when converting from SVG, `Z` is converted to
`L` (if the current point is further from the initial point than the given precision)
and `Z`.

`Z` is extremely useful when converting between SVG and SvgCom multiple times.
Specifically, the fact of a path being closed can be used when cutting invisible (covered) path segments. -->

### Example 0

`example.svgcom`
```
720 480 2 4
MLZ
0 0 100 100 0 0
```

This file defines an image of size `720`x`480`
with `2` commands (`M`ove and `L`ine and ClosePath)
and `4` coordinates (`0.0`, `0.0`, `100.0`, `100.0`, `0.0` and `0.0`), which form three points.

`Z` commands always

### Example 1

`input.svg`
```xml
<?xml version="1.0" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 250 200">
    <path stroke="#000000" fill="none" d="M 0 0 L 100 0 L 100 70 L 0 70 Z"/>
</svg>
```
`output.svgcom`
```
800 600 5 10
MLLLZ
0 0 100 0 100 70 0 70 0 0
```

### Example 2

`input.svg`
```xml
<?xml version="1.0" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
    <!-- Hidden elements are removed -->
    <path stroke="#000000" visibility="hidden" fill="none" d="M 0 0 L 100 0 L 100 70 L 0 70 Z"/>

    <!-- Groups get flattened -->
    <g>
        <g>
            <!-- All shapes are converted to paths -->
            <ellipse cx="100" cy="100" rx="50" ry="60"/>
        </g>
    </g>
</svg>
```

`output.svgcom`
```
800 600 6 28
MCCCCL
150 100 150 133.1370849898476 127.61423749153967 160 100 160 72.38576250846033 160 50.00000000000001 133.1370849898476 50 100.00000000000001 49.99999999999999 66.86291501015242 72.38576250846032 39.99999999999999 99.99999999999999 39.999999999999986 127.61423749153965 39.99999999999997 150 66.86291501015239 150 100 150 100
```