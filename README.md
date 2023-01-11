# SVG Path Simplifier

This program converts SVG images into MoveTo/LineTo/CubixBezierTo commands.

### Usage

```sh
svgps INPUT.svg OUTPUT.svgpath
```

### Output format

```
VIEWBOX_WIDTH VIEWBOX_HEIGHT
NUMBER_OF_COORDINATES NUMBER_OF_COMMANDS
<float64>...
<"M" | "L" | "C">...
```

### Example

`input.svg`
```xml
<?xml version="1.0" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 250 200">
    <path stroke="#000000" fill="none" d="M 0 0 L 100 0 L 100 70 L 0 70 Z"/>
</svg>
```
`output.svgpath`
```
800 600
10 5
0 0 100 0 100 70 0 70 0 0
M L L L L
```