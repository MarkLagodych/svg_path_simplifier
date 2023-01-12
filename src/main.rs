#![allow(unused)]

extern crate usvg;

use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
enum PathCommand {
    Move,
    Line,
    /// Cubic Bezier Path
    Curve,
}

#[derive(Debug)]
struct PathData {
    pub view_size: (f64, f64),
    pub coordinates: Vec<f64>,
    pub commands: Vec<PathCommand>
}


const TEXT_USAGE: &'static str =
r#"Usage:
    svgps help
    svgps generate INPUT.svg OUTPUT.svgpath
    svgps render INPUT.svgpath OUTPUT.svg
"#;

const TEXT_HELP: &'static str = 
r#"SVG Path Simplifier
by Mark Lagodych <https://github.com/MarkLagodych/svg_path_simplifier>

This program converts SVG art into MoveTo/LineTo/CubicBezierCurveTo commands.
The goal is to make SVG easier to understand for plotters."#;


fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() != 4 {
        println!("{}", TEXT_USAGE);
        return;
    }

    let action = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    let input = std::fs::read_to_string(input_path).expect("Cannot read input file");
    let mut output = File::create(output_path).expect("Cannot open output file");

    match action.as_str() {
        "generate" => generate(&input, &mut output),

        "render" => render(&input, &mut output),

        _ => {
            println!()
        }
    }
}


fn generate(input: &str, mut output: &mut File) {
    let svg = usvg::Tree::from_str(&input, &usvg::Options::default()).unwrap();

    let mut path_data = PathData::new();

    path_data.view_size = (svg.view_box.rect.width(), svg.view_box.rect.height());

    for node in svg.root.descendants() {
        collect_path(&node, &mut path_data);
    }

    write_path(&path_data, &mut output);
}


fn render(input: &str, mut output: &mut File) {
    let mut lines = input.lines();

    let metrics = lines.next().unwrap()
        .split_whitespace()
        .map(|x| x.parse::<u32>().unwrap())
        .collect::<Vec<u32>>();

    let mut coords = lines.next().unwrap()
        .split_whitespace()
        .map(|x| x.parse::<f64>().unwrap());

    let mut commands = lines.next().unwrap()
        .chars();

    writeln!(output, r#"<?xml version="1.0" standalone="no"?>"#);

    writeln!(
        output,
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        metrics[0], metrics[1]
    );

    output.write(br##"<path stroke="#000000" fill="none" d=""##);

    for cmd in commands {
        match cmd {
            'M' => {
                let x = coords.next().unwrap();
                let y = coords.next().unwrap();
                write!(output, "M {x} {y} ");
            }

            'L' => {
                let x = coords.next().unwrap();
                let y = coords.next().unwrap();
                write!(output, "L {x} {y} ");
            }

            'C' => {
                let x0 = coords.next().unwrap();
                let y0 = coords.next().unwrap();
                let x1 = coords.next().unwrap();
                let y1 = coords.next().unwrap();
                let x2 = coords.next().unwrap();
                let y2 = coords.next().unwrap();
                write!(output, "C {x0} {y0}, {x1} {y1}, {x2} {y2} ");
            }

            _ => {}
        }
    }

    output.write(b"\"/>\n");

    output.write(b"</svg>");

}


impl std::fmt::Display for PathCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            PathCommand::Move => "M",
            PathCommand::Line => "L",
            PathCommand::Curve => "C"
        })
    }
}


impl PathData {
    pub fn new() -> Self {
        Self {
            view_size: (0., 0.),
            coordinates: vec![],
            commands: vec![]
        }
    }
}


fn collect_path(svg_node: &usvg::Node, output: &mut PathData) {
    if let usvg::NodeKind::Path(ref path) = *svg_node.borrow() {
        if path.visibility != usvg::Visibility::Visible {
            return;
        }

        let mut coordinates = path.data.points().iter();

        let mut initial_point = (0.0f64, 0.0f64);

        for command in path.data.commands() {
            match *command {
                usvg::PathCommand::MoveTo => {
                    let x = *coordinates.next().unwrap();
                    let y = *coordinates.next().unwrap();
                    initial_point = (x, y);
                    output.coordinates.push(x);
                    output.coordinates.push(y);
                    output.commands.push(PathCommand::Move);
                }

                usvg::PathCommand::LineTo => {
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.commands.push(PathCommand::Line);
                }

                usvg::PathCommand::CurveTo => {
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.coordinates.push(*coordinates.next().unwrap());
                    output.commands.push(PathCommand::Curve);
                }

                usvg::PathCommand::ClosePath => {
                    output.coordinates.push(initial_point.0);
                    output.coordinates.push(initial_point.1);
                    output.commands.push(PathCommand::Line);
                }
            }
        }
    }

}


fn write_path(data: &PathData, mut file: &mut File) {
    write!(file, "{} {} ", data.view_size.0, data.view_size.1);

    write!(file, "{} {}\n", data.coordinates.len(), data.commands.len());

    for (i, coord) in data.coordinates.iter().enumerate() {
        if i != 0 { write!(file, " "); }
        
        write!(file, "{}", coord);
    }

    write!(file, "\n");

    for cmd in data.commands.iter() {
        write!(file, "{}", cmd);
    }
}