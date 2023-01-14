use crate::{
    Error,
    GenerateArgs,
    RenderArgs
};

use std::{
    cell::Ref,
    fs::File,
    path::PathBuf,
    collections::HashMap,
    io::prelude::*
};


/// .svg path representation
///
/// Reference-counted pointer
#[derive(Clone)]
struct SvgPath(usvg::Node);


/// Intermediate path representation that is easier to process
struct Path {
    pub source: SvgPath,

    /// Elemementary path segments: lines and Bezier curves
    pub segments: Vec<kurbo::PathSeg>,
}


struct PathIntersections {
    /// Segment index -> its intersections with another path
    pub info: HashMap<usize, Vec<kurbo::LineIntersection>>,
}


struct ImageSize {
    width: u32,
    height: u32,
}

/// Final .svgcom representation
struct SvgCom {
    pub view_size: ImageSize,
    pub commands: kurbo::BezPath,
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
    let svg_paths = get_svg_paths(&svg);

    let mut svgcom = SvgCom::new(svg.size.width(), svg.size.height());

    if !args.autocut {

        svgcom.read_from_svg_paths(&svg_paths);
        
    } else {

        todo!("Autocut not implemented")

    }

    write!(output, "{}", svgcom.to_svgcom());

    Ok(())
}


pub fn render_to_svg(args: RenderArgs) -> Result<(), Error> {
    let input = read_file(&args.input)?;
    let mut output = open_writable_file(&args.output)?;

    let svgcom = SvgCom::from_svgcom_str(&input)?;

    write_svg_start(&mut output, &args, &svgcom.view_size);

    write!(output, "{}", svgcom.to_svg_path_data());

    write_svg_end(&mut output);

    Ok(())
}


fn parse_svg(input: &str) -> Result<usvg::Tree, Error> {
    usvg::Tree::from_str(&input, &usvg::Options::default())
        .or_else(|err| Err(format!("Cannot parse SVG: {}", err.to_string())))
}


fn get_svg_paths(svg: &usvg::Tree) -> Vec<SvgPath> {
    svg.root.descendants()
        .filter(|node| 
            match *node.borrow() {
                usvg::NodeKind::Path(_) => true,
                _ => false
            })
        .map(|node| SvgPath::new(&node))
        .filter(|path_result| path_result.is_some())
        .map(|result| result.unwrap())
        .collect::<Vec<SvgPath>>()
}


fn write_svg_start(output: &mut File, args: &RenderArgs, size: &ImageSize) {
    writeln!(output, r#"<?xml version="1.0" standalone="no"?>"#);

    writeln!(
        output,
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        size.width, size.height
    );

    write!(
        output,
        r##"<path stroke="{}" stroke-width="{}" fill="none" d=""##,
        args.stroke,
        args.stroke_width
    );
}


fn write_svg_end(output: &mut File) {
    writeln!(output, r#""/>"#);

    write!(output, "</svg>");
}


impl SvgPath {
    /// Returns [None] if the node is not a [usvg::Path]
    pub fn new(node: &usvg::Node) -> Option<Self> {
        if let usvg::NodeKind::Path(_) = *node.borrow() {
            return Some(Self(node.clone()))
        }

        None
    }


    pub fn borrow(&self) -> Ref<'_, usvg::Path> {
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
        let path = self.borrow();

        return path.data.commands().len() > 0
            && *path.data.commands().last().unwrap() == usvg::PathCommand::ClosePath
    }


    /// Additional test for checking whether a given point lies inside a shape
    pub fn test_winding(&self, winding: i32) -> bool {
        let path = self.borrow();

        if let Some(fill) = &path.fill {
            match fill.rule {
                usvg::FillRule::EvenOdd => return winding % 2 == 0,
                usvg::FillRule::NonZero => return winding != 0,
            }
        }

        return false
    }


}


impl ImageSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}


impl SvgCom {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            view_size: ImageSize::new(width.ceil() as u32, height.ceil() as u32),
            commands: kurbo::BezPath::new()
        }
    }


    pub fn read_from_svg_paths(&mut self, svg_paths: &Vec<SvgPath>) {
        for path in svg_paths {
            self.read_from_svg_path(path);
        }
    }


    pub fn from_svgcom_str(source: &str) -> Result<Self, Error> {
        let lines = source.lines()
            .collect::<Vec<&str>>();

        if lines.len() < 3 {
            return Err("Expected at least 3 lines".to_string());
        }

        let mut lines = lines.iter();

        let metrics = Self::parse_svgcom_metrics(lines.next().unwrap())?;
        let commands = Self::parse_svgcom_commands(lines.next().unwrap());
        let coords = Self::parse_svgcom_coords(lines.next().unwrap())?;

        if commands.len() != metrics[2] as usize || coords.len() != metrics[3] as usize {
            return Err("Data length does not match the header information".to_string());
        }

        let mut me = Self::new(metrics[0] as f64, metrics[1] as f64);

        me.read_svgcom_data(commands, coords)?;

        Ok(me)
    }


    fn parse_svgcom_metrics(line: &str) -> Result<Vec<u32>, Error> {
        let metrics = line
            .split_whitespace()
            .map(|x| x.parse::<u32>()
                .map_err(|err| format!("Uint32 parsing error: {}", err)))
            .collect::<Result<Vec<u32>, Error>>()?;

        if metrics.len() != 4 {
            return Err("Expected 4 metrics components: WIDTH, HEIGHT, N_CMD, N_COORD".to_string());
        }

        Ok(metrics)
    }

    fn parse_svgcom_commands(line: &str) -> Vec<char> {
        line
            .chars()
            .collect::<Vec<char>>()
    }

    fn parse_svgcom_coords(line: &str) -> Result<Vec<f64>, Error> {
        line
            .split_whitespace()
            .map(|x| x.parse::<f64>()
                .map_err(|err| format!("Float64 parsing error: {}", err)))
            .collect::<Result<Vec<f64>, Error>>()
    }


    fn read_svgcom_data(&mut self, commands: Vec<char>, coords: Vec<f64>) -> Result<(), Error> {
        let mut coords_iter = coords.iter();
        let mut get_point = || {
            kurbo::Point::new(*coords_iter.next().unwrap(), *coords_iter.next().unwrap())
        };

        for cmd in commands {
            match cmd {
                'M' => self.commands.move_to(get_point()),
                'L' => self.commands.line_to(get_point()),
                'C' => self.commands.curve_to(get_point(), get_point(), get_point()),
                'Z' => self.commands.close_path(),
                c => return Err(format!("Invalid command: {}", c)),
            }
        }

        Ok(())
    }

    pub fn points_count(&self) -> usize {
        let mut npoints = 0;

        for cmd in self.commands.iter() {
            npoints += match cmd {
                kurbo::PathEl::MoveTo(_) => 1,
                kurbo::PathEl::LineTo(_) => 1,
                kurbo::PathEl::CurveTo(_, _, _) => 3,
                kurbo::PathEl::ClosePath => 0,
                kurbo::PathEl::QuadTo(_, _) => panic!("unexpected quadradic curve"),
            };
        }

        npoints
    }


    pub fn coordinates_count(&self) -> usize {
        self.points_count() * 2
    }


    pub fn to_svg_path_data(&self) -> String {
        struct A<'a>(&'a SvgCom);

        impl<'a> std::fmt::Display for A<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.format_svg_path_data(f)
            }
        }

        format!("{}", A(self))
    }


    pub fn to_svgcom(&self) -> String {
        struct A<'a>(&'a SvgCom);

        impl<'a> std::fmt::Display for A<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.format_svgcom(f)
            }
        }

        format!("{}", A(self))
    }

    pub(self) fn format_svg_path_data(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for cmd in self.commands.iter() {
            match cmd {
                kurbo::PathEl::MoveTo(p) =>
                    write!(f, "M{} {}", p.x, p.y)?,

                kurbo::PathEl::LineTo(p) =>
                    write!(f, "L{} {}", p.x, p.y)?,

                kurbo::PathEl::CurveTo(p1, p2, p3) =>
                    write!(f, "C{} {},{} {},{} {}", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)?,

                kurbo::PathEl::ClosePath =>
                    write!(f, "Z")?,

                _ => {}
            }
        }

        Ok(())
    }


    fn read_from_svg_path(&mut self, svg_path: &SvgPath) {
        let path = svg_path.borrow();
        let path_data = &path.data;
        let mut coords = path_data.points().iter();
        let commands = path_data.commands().iter();

        let mut get_point = || {
            let mut x = *(coords.next().unwrap());
            let mut y = *(coords.next().unwrap());
            path.transform.apply_to(&mut x, &mut y);
            kurbo::Point::new(x, y)
        };

        for command in commands {
            match *command {
                usvg::PathCommand::MoveTo => self.commands.move_to(get_point()),
                usvg::PathCommand::LineTo => self.commands.line_to(get_point()),
                usvg::PathCommand::CurveTo => self.commands.curve_to(get_point(), get_point(), get_point()),
                usvg::PathCommand::ClosePath => self.commands.close_path(),
            }
        }
    }

    


    pub(self) fn format_svgcom(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.format_svgcom_metrics(f)?;
        self.format_svgcom_commands(f)?;
        self.format_svgcom_points(f)?;
        Ok(())
    }


    fn format_svgcom_metrics(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {} {}",
            self.view_size.width,
            self.view_size.height,
            self.commands.elements().len(),
            self.coordinates_count()
        )
    }


    fn format_svgcom_commands(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for cmd in self.commands.iter() {
            match cmd {
                kurbo::PathEl::MoveTo(_) => write!(f, "M")?,
                kurbo::PathEl::LineTo(_) => write!(f, "L")?,
                kurbo::PathEl::CurveTo(_, _, _) => write!(f, "C")?,
                kurbo::PathEl::ClosePath => write!(f, "Z")?,
                _ => {}
            }
        }

        writeln!(f, "")?;

        Ok(())
    }


    fn format_svgcom_points(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, cmd) in self.commands.iter().enumerate() {
            if i != 0 { write!(f, " ")?; }

            match cmd {
                kurbo::PathEl::MoveTo(p) =>
                    write!(f, "{} {}", p.x, p.y)?,

                kurbo::PathEl::LineTo(p) =>
                    write!(f, "{} {}", p.x, p.y)?,

                kurbo::PathEl::CurveTo(p1, p2, p3) =>
                    write!(f, "{} {} {} {} {} {}", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)?,

                _ => {}
            }
        }

        writeln!(f, "")?;
        
        Ok(())
    }
}