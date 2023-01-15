use crate::{
    Error,
    GenerateArgs,
    RenderArgs,

    svgcom::*
};

use std::{
    rc::Rc,
    cell::Ref,
    fs::File,
    path::PathBuf,
    collections::HashMap,
    io::prelude::*, borrow::Borrow, 
};


/// .svg path representation
///
/// Reference-counted pointer
#[derive(Clone)]
pub struct SvgPathNode(usvg::Node);


pub struct SvgPathPoints {
    path_data: Rc<usvg::PathData>,
    transform: usvg::Transform,
    coordinate_index: usize,
}

pub struct SvgPathCommands {
    path_data: Rc<usvg::PathData>,
    command_index: usize,
}



/// Intermediate path representation that is easier to process
pub struct Path {
    pub source: SvgPathNode,

    /// Elemementary path segments: lines and Bezier curves
    pub segments: Vec<kurbo::PathSeg>,
}


struct PathIntersections {
    /// Segment index -> its intersections with another path
    pub info: HashMap<usize, Vec<kurbo::LineIntersection>>,
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
    let svg_path_nodes = get_svg_paths(&svg, &args);

    let mut svgcom = SvgCom::new(svg.size.width(), svg.size.height());

    if !args.autocut {

        svgcom.read_from_svg_paths(&svg_path_nodes);
        
    } else {

        let paths = svg_path_nodes.iter()
            .map(|node| Path::new(&node))
            .collect::<Vec<Path>>();

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


fn get_svg_paths(svg: &usvg::Tree, args: &GenerateArgs) -> Vec<SvgPathNode> {
    svg.root.descendants()
        .filter(|node| 
            match *node.borrow() {
                usvg::NodeKind::Path(_) => true,
                _ => false
            })
        .map(|node| SvgPathNode::new(&node))
        .filter(|path_result| path_result.is_some())
        .map(|result| result.unwrap())
        .filter(|path| !args.onlystroked || path.get_path().stroke.is_some())
        .collect::<Vec<SvgPathNode>>()
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


impl SvgPathNode {
    /// Returns [None] if the node is not a [usvg::Path]
    pub fn new(node: &usvg::Node) -> Option<Self> {
        if let usvg::NodeKind::Path(_) = *node.borrow() {
            return Some(Self(node.clone()))
        }

        None
    }


    pub fn get_path(&self) -> Ref<'_, usvg::Path> {
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
        let path = self.get_path();

        return path.data.commands().len() > 0
            && *path.data.commands().last().unwrap() == usvg::PathCommand::ClosePath
    }


    /// Additional test for checking whether a given point lies inside a shape
    pub fn test_winding(&self, winding: i32) -> bool {
        let path = self.get_path();

        if let Some(fill) = &path.fill {
            match fill.rule {
                usvg::FillRule::EvenOdd => return winding % 2 == 0,
                usvg::FillRule::NonZero => return winding != 0,
            }
        }

        return false
    }


    pub fn get_points_iter(&self) -> SvgPathPoints {
        SvgPathPoints::from(self)
    }

    pub fn get_commands_iter(&self) -> SvgPathCommands {
        SvgPathCommands::from(self)
    }

}


impl Path {
    pub fn new(svg_path: &SvgPathNode) -> Self {
        Self {
            source: svg_path.clone(),
            segments: Self::get_path_segments(&svg_path)
        }
    }


    fn get_path_segments(svg_node: &SvgPathNode) -> Vec<kurbo::PathSeg> {
        let mut bezpath = kurbo::BezPath::new();

        let mut commands = svg_node.get_commands_iter();
        let mut points = svg_node.get_points_iter();
        let mut p = || points.next().unwrap();
        
        for command in commands {
            match command {
                usvg::PathCommand::MoveTo => bezpath.move_to(p()),
                usvg::PathCommand::LineTo => bezpath.line_to(p()),
                usvg::PathCommand::CurveTo => bezpath.curve_to(p(), p(), p()),
                usvg::PathCommand::ClosePath => bezpath.close_path(),
            }
        }

        bezpath.segments()
            .collect::<Vec<kurbo::PathSeg>>()
    }
}



impl SvgPathPoints {
    pub fn from(svg_node: &SvgPathNode) -> Self {
        
        let path = svg_node.get_path();
        let path_data = Rc::clone(&path.data);
        let transform = path.transform.clone();

        Self {
            path_data,
            transform,
            coordinate_index: 0,
        }
    }
}


impl SvgPathCommands {
    pub fn from(svg_node: &SvgPathNode) -> Self {
        
        let path = svg_node.get_path();
        let path_data = Rc::clone(&path.data);

        Self {
            path_data,
            command_index: 0,
        }
    }
}


impl Iterator for SvgPathPoints {
    type Item = kurbo::Point;

    fn next(&mut self) -> Option<Self::Item> {
        let coordinates = self.path_data.points();
        let x = *coordinates.get(self.coordinate_index)?;
        let y = *coordinates.get(self.coordinate_index + 1)?;
        self.coordinate_index += 2;

        let (x, y) = self.transform.apply(x, y);

        Some(kurbo::Point::new(x, y))
    }
}


impl Iterator for SvgPathCommands {
    type Item = usvg::PathCommand;

    fn next(&mut self) -> Option<Self::Item> {
        let command = self.path_data.commands().get(self.command_index)?.clone();
        self.command_index += 1;
        Some(command)
    }
}