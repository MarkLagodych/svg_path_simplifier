use crate::{
    Error,
    GenerateArgs,
    RenderArgs
};

use std::{
    cell::Ref,
    fs::File,
    path::PathBuf,
    io::prelude::*
};


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


/// Reference-counted pointer
struct SvgPath(usvg::Node);


struct PathSegment {
    pub source: SvgPath,

    /// Elemementary path segments: lines and Bezier curves
    pub subsegments: kurbo::PathSeg,
}


fn read_file(path: &PathBuf) -> Result<String, Error> {
    std::fs::read_to_string(path)
        .or_else(|msg|
            Err(format!(r#"Cannot read file "{}": {}"#, path.to_string_lossy(), msg))
        )
}


fn open_writable_file(path: &PathBuf) -> Result<File, Error> {
    File::create(path)
        .or_else(|msg|
            Err(format!(r#"Cannot open file "{}": {}"#, path.to_string_lossy(), msg))
        )
}



pub fn generate_from_svg(args: GenerateArgs) -> Result<(), Error> {
    let input = read_file(&args.input)?;
    let mut output = open_writable_file(&args.output)?;

    let svg = parse_svg(&input)?;

    let mut path_data = PathData::new();

    path_data.view_size = (svg.view_box.rect.width(), svg.view_box.rect.height());

    for node in svg.root.descendants() {
        collect_path(&node, &mut path_data);
    }

    write_path(&path_data, &mut output);

    Ok(())
}


pub fn render_to_svg(args: RenderArgs) -> Result<(), Error> {
    let input = read_file(&args.input)?;
    let mut output = open_writable_file(&args.output)?;

    let mut lines = input.lines();

    let metrics = lines.next().unwrap()
        .split_whitespace()
        .map(|x| x.parse::<u32>().unwrap())
        .collect::<Vec<u32>>();

    let mut coords = lines.next().unwrap()
        .split_whitespace()
        .map(|x| x.parse::<f64>().unwrap());

    let commands = lines.next().unwrap()
        .chars();

    writeln!(output, r#"<?xml version="1.0" standalone="no"?>"#);

    writeln!(
        output,
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        metrics[0], metrics[1]
    );

    write!(output, r##"<path stroke="#000000" fill="none" d=""##);

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

    writeln!(output, r#""/>"#);

    write!(output, "</svg>");

    Ok(())
}


fn parse_svg(input: &str) -> Result<usvg::Tree, Error> {
    usvg::Tree::from_str(&input, &usvg::Options::default())
        .or_else(|err| Err(format!("Cannot parse SVG: {}", err.to_string())))
}


impl SvgPath {
    /// Returns [None] if the node is not a [usvg::Path]
    pub fn new(node: &usvg::Node) -> Option<Self> {
        if let usvg::NodeKind::Path(_) = *node.borrow() {
            return Some(Self(node.clone()))
        }

        None
    }


    pub fn get_svg_path(&self) -> Ref<'_, usvg::Path> {
        let node_ref = self.0.borrow();
        return Ref::map(node_ref, |node_ref| {
            if let usvg::NodeKind::Path(ref path) = node_ref {
                path
            }
            else {
                panic!("SvgPath's node is not a path!")
            }
        });

    }


    pub fn is_closed(&self) -> bool {
        let path = self.get_svg_path();

        return path.data.commands().len() > 0
            && *path.data.commands().last().unwrap() == usvg::PathCommand::ClosePath
    }


    /// Returns true if the winding test passes
    pub fn test_winding(&self, winding: i32) -> bool {
        let path = self.get_svg_path();

        if let Some(fill) = &path.fill {
            match fill.rule {
                usvg::FillRule::EvenOdd => return winding % 2 == 0,
                usvg::FillRule::NonZero => return winding != 0,
            }
        }

        return false
    }


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
        let mut last_point = (0.0f64, 0.0f64);

        let mut get_point = || -> (f64, f64) {
            let x = *coordinates.next().unwrap();
            let y = *coordinates.next().unwrap();
            return path.transform.apply(x, y)
        };

        let mut push_point = |p: (f64, f64)| {
            output.coordinates.push(p.0);
            output.coordinates.push(p.1);
        };

        for command in path.data.commands() {
            match *command {
                usvg::PathCommand::MoveTo => {
                    let p = get_point();
                    initial_point = p.clone();
                    last_point = p.clone();
                    push_point(p);

                    output.commands.push(PathCommand::Move);
                }

                usvg::PathCommand::LineTo => {
                    let p = get_point();
                    last_point = p.clone();
                    push_point(p);

                    output.commands.push(PathCommand::Line);
                }

                usvg::PathCommand::CurveTo => {
                    push_point(get_point());
                    push_point(get_point());

                    let p = get_point();
                    last_point = p.clone();
                    push_point(p);

                    output.commands.push(PathCommand::Curve);
                }

                usvg::PathCommand::ClosePath => {
                    // If there is nothing to draw, skip the command
                    if last_point == initial_point {
                        continue;
                    }

                    push_point(initial_point.clone());
                    output.commands.push(PathCommand::Line);
                }
            }
        }
    }

}


fn write_path(data: &PathData, file: &mut File) {
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