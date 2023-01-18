# SVG Path Simplifier

The `svgps` program converts `.svg` images into MoveTo/LineTo/CubicBezierTo/ClosePath commands
contained in a [`.svgcom`](#svgcom-format) file.

The initial goal was to prettify and simplify SVG before sending to plotters.

It utilizes [usvg](https://github.com/RazrFalcon/resvg/tree/master/usvg) for SVG parsing and simplification
and [kurbo](https://github.com/linebender/kurbo) for geometric calculations.

## Demos

| `XXXXXXXXXXXXXXXXX` | `XXXXXXXXXXXXXXXXX` | `XXXXXXXXXXXXXXXXX` |
| --- | --- | --- |
| Original SVG | Generated (untouched) | Generated (autocut) |
| ![ferris](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/ferris.svg) | ![ferris-gen](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/ferris-converted.svg) | ![ferris-cut](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/ferris-converted-autocut.svg) |
| ![tiger](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/tiger.svg) | ![tiger-gen](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/tiger-converted.svg) | ![tiger-cut](https://github.com/MarkLagodych/svg_path_simplifier/blob/main/demo/tiger-converted-autocut.svg) |

See [demo/gen.sh](./demo/gen.sh).

## Features

* Generating `.svgcom` from `.svg`
* Rendering `.svgcom` to `.svg` for previewing
* Selecting only stroked paths for the conversion
* Autocutting path segments that are not visible because of being covered by other figures
  (think of this as of a depth test)

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

### Example 0

`example.svgcom`
```
720 480 2 4
MLZ
0 0 100 100
```

This file defines an image of size `720`x`480`
with `2` commands (`M`ove and `L`ine and ClosePath)
and `4` coordinates (`0.0`, `0.0`, `100.0`, `100.0`), which form two points.

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
0 0 100 0 100 70 0 70
```

### Example 2

`input.svg`
```xml
<?xml version="1.0" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
    <!-- Hidden elements are removed -->
    <path stroke="#000000" visibility="hidden" fill="none" d="M 0 0 L 100 0 L 100 70 L 0 70 Z"/>

    <!-- Groups are flattened -->
    <g>
        <!-- Transforms are resolved -->
        <g transform="translate(0 0)">
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