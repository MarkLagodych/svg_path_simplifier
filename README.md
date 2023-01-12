# SVG Path Simplifier

This program converts SVG images into MoveTo/LineTo/CubicBezierTo commands.

It utilizes the [usvg](https://github.com/RazrFalcon/resvg/tree/master/usvg) crate for SVG parsing and simplification.

### Usage

```sh
svgps generate INPUT.svg OUTPUT.svgpath
svgps render INPUT.svgpath OUTPUT.svg
```

### SvgPath format

```
<viewbox_width: uint32>
<SPACE>
<viewbox_height: uint32>
<SPACE>
<number_of_coordinates: uint32>
<SPACE>
<number_of_commands: uint32>
<EOL>

<coordinate: float64>... (delimited by <SPACE>)
<EOL>

<command: "M" | "L" | "C">...
```

### Example 0

`example.svgpath`
```
720 480 4 2
0 0 100 100
ML
```

This file defines an image of size `720`x`480`
with `4` coordinates (`0.0`, `0.0`, `100.0` and `100.0`), which form two points,
and `2` commands: `M`ove and `L`ine.

### Example 1

`input.svg`
```xml
<?xml version="1.0" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 250 200">
    <path stroke="#000000" fill="none" d="M 0 0 L 100 0 L 100 70 L 0 70 Z"/>
</svg>
```
`output.svgpath`
```
800 600 10 5
0 0 100 0 100 70 0 70 0 0
MLLLL
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

`output.svgpath`
```
800 600 28 6
150 100 150 133.1370849898476 127.61423749153967 160 100 160 72.38576250846033 160 50.00000000000001 133.1370849898476 50 100.00000000000001 49.99999999999999 66.86291501015242 72.38576250846032 39.99999999999999 99.99999999999999 39.999999999999986 127.61423749153965 39.99999999999997 150 66.86291501015239 150 100 150 100
MCCCCL
```