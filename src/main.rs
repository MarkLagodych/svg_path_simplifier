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


fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() != 3 {
        println!("Usage: svgps INPUT.svg OUTPUT.txt");
        return;
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let input = std::fs::read_to_string(input_path).expect("Cannot read input file");
    let mut output = File::create(output_path).expect("Cannot open output file");

    let svg = usvg::Tree::from_str(&input, &usvg::Options::default()).unwrap();

    let mut path_data = PathData::new();

    path_data.view_size = (svg.view_box.rect.width(), svg.view_box.rect.height());

    for node in svg.root.descendants() {
        collect_path(&node, &mut path_data);
    }

    write_path(&path_data, &mut output);
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
    match *svg_node.borrow() {
        usvg::NodeKind::Path(ref path) => {
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
                        output.commands.push(PathCommand::Move);
                    }

                    usvg::PathCommand::ClosePath => {
                        output.coordinates.push(initial_point.0);
                        output.coordinates.push(initial_point.1);
                        output.commands.push(PathCommand::Line);
                    }
                }
            }
        }

        _ => {}
    }
}


fn write_path(data: &PathData, mut file: &mut File) {
    write!(file, "{} {}\n", data.view_size.0, data.view_size.1);

    write!(file, "{} {}\n", data.coordinates.len(), data.commands.len());

    for (i, coord) in data.coordinates.iter().enumerate() {
        if i != 0 { write!(file, " "); }
        
        write!(file, "{}", coord);
    }

    write!(file, "\n");

    for (i, cmd) in data.commands.iter().enumerate() {
        if i != 0 { write!(file, " "); }
        
        write!(file, "{}", cmd);
    }

    write!(file, "\n");
}