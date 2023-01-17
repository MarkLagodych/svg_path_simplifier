use kurbo::{ParamCurve, Shape};

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
/// Reference-counted
#[derive(Clone)]
pub struct SvgPathNode {
    node: usvg::Node,
}


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
#[derive(Clone)]
pub struct Path {
    pub source: SvgPathNode,

    /// Elemementary path segments: lines and Bezier curves
    pub segments: Vec<kurbo::PathSeg>,
}


/// Segment index -> segment's intersections with another path
type PathIntersections = HashMap<usize, Vec<kurbo::LineIntersection>>;


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
            .map(|node| Path::from(&node))
            .collect::<Vec<Path>>();

        let paths = cut_paths(&paths);

        svgcom.read_from_paths(&paths);

    }

    write!(output, "{}", svgcom.to_svgcom_str());

    Ok(())
}


pub fn render_to_svg(args: RenderArgs) -> Result<(), Error> {
    let input = read_file(&args.input)?;
    let mut output = open_writable_file(&args.output)?;

    let svgcom = SvgCom::from_svgcom_str(&input)?;

    write_svg_start(&mut output, &args, &svgcom.view_size);

    write!(output, "{}", svgcom.to_svg_path_data_str());

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
        r##"<path stroke="{}" stroke-width="{}" fill="{}" d=""##,
        args.stroke,
        args.stroke_width,
        args.fill
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
            let mut me = Self {
                node: node.clone(),
            };


            return Some(me)
        }

        None
    }


    pub fn get_path(&self) -> Ref<'_, usvg::Path> {
        let node_ref = self.node.borrow();
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


    pub fn can_cut(&self) -> bool {
        self.is_closed() && self.get_path().fill.is_some()
    }


    /// Test for checking whether a given point lies inside a shape
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


impl PartialEq for SvgPathNode {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}


impl Path {
    /// Constructs an empty path
    pub fn new(svg_path: &SvgPathNode) -> Self {
        Self {
            source: svg_path.clone(),
            segments: vec![]
        }
    }


    /// Copies path segments
    pub fn from(svg_path: &SvgPathNode) -> Self {
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


fn get_segment_intersection(
    intersected: &kurbo::PathSeg,
    intersecting: &kurbo::PathSeg,
    precision: f64
) -> Vec<kurbo::LineIntersection> {
    match intersecting {
        kurbo::PathSeg::Line(line) =>
            get_line_intersection(intersected, line, precision),

        kurbo::PathSeg::Cubic(curve) =>
            get_curve_intersection(intersected, curve, precision),
        
        _ => 
            vec![]
    }
}


fn get_line_intersection(
    intersected: &kurbo::PathSeg,
    intersecting: &kurbo::Line,
    precision: f64
) -> Vec::<kurbo::LineIntersection> {
    return intersected
        .intersect_line(intersecting.clone())
        .to_vec()
}


fn get_curve_intersection(
    intersected: &kurbo::PathSeg,
    intersecting: &kurbo::CubicBez,
    precision: f64
) -> Vec::<kurbo::LineIntersection> {
    let points = curve_to_points(&intersecting, precision);
    let line_starts = points[..points.len()-1].iter();
    let line_ends = points[1..].iter();

    let mut intersections = Vec::<kurbo::LineIntersection>::new();

    for (line_start, line_end) in line_starts.zip(line_ends) {
        let line = kurbo::Line::new(*line_start, *line_end);

        intersections.extend(
            intersected.intersect_line(line)
        );
    }

    return intersections
}


fn curve_to_points(curve: &kurbo::CubicBez, precision: f64) -> Vec<kurbo::Point> {
    use kurbo::Shape;

    let mut points = Vec::<kurbo::Point>::new();

    curve
        .into_path(precision)
        .flatten(precision, |path_element|
            match path_element {
                kurbo::PathEl::MoveTo(point) => points.push(point),
                kurbo::PathEl::LineTo(point) => points.push(point),
                _ => {}
            }
        );

    points
}


fn cut_segment(
    segment: &kurbo::PathSeg,
    intersections: &Vec<kurbo::LineIntersection>
) -> Vec::<kurbo::PathSeg> {

    let mut last_t = 0f64;

    let mut subsegments = Vec::<kurbo::PathSeg>::new();

    for intersection in intersections {
        let segment_t = intersection.segment_t;
        let subsegment = segment.subsegment(last_t..segment_t);
        subsegments.push(subsegment);
        last_t = segment_t;
    }

    subsegments.push(segment.subsegment(last_t..1.0));

    subsegments

}


fn get_path_intersections(
    intersected: &Path,
    intersecting: &Path,
    precision: f64
) -> PathIntersections {

    let mut path_intersections = PathIntersections::new();

    for (segment_index, intersected_segment) in intersected.segments.iter().enumerate() {
        for intersecting_segment in intersecting.segments.iter() {
            let segment_intersections = get_segment_intersection(
                intersected_segment,
                intersecting_segment,
                precision
            );

            if !segment_intersections.is_empty() {
                path_intersections.insert(segment_index, segment_intersections);
            }
        }
    }

    path_intersections
}


fn cut_path(path: &Path, intersections: &PathIntersections) -> Vec<Path> {

    let mut subpaths = Vec::<Path>::new();

    let new_path = || Path::new(&path.source);

    subpaths.push(new_path());

    for (index, segment) in path.segments.iter().enumerate() {
        if !intersections.contains_key(&index) {
            subpaths.last_mut().unwrap().segments.push(segment.clone());
        } else {
            let _subsegments = cut_segment(segment, &intersections[&index]);
            let mut subsegments = _subsegments.iter();

            if let Some(subsegment) = subsegments.next() {
                subpaths.last_mut().unwrap().segments.push(segment.clone());
            }

            while let Some(subsegment) = subsegments.next() {
                subpaths.push(new_path());
                subpaths.last_mut().unwrap().segments.push(segment.clone());
            }
        }
    }

    subpaths

}


fn is_path_covered(covered: &Path, covering: &Path) -> bool {
    if covered.segments.len() == 0 || covering.segments.len() == 0 {
        return false;
    }

    if covered.source == covering.source {
        return false;
    }

    let some_point = covered.segments[0].eval(0.5);

    is_point_covered(some_point, covering)
}


fn is_point_covered(point: kurbo::Point, covering: &Path) -> bool {

    /// TODO do not recalculate this every time
    let bezpath = kurbo::BezPath::from_path_segments(covering.segments.clone().into_iter());

    covering.source.test_winding(bezpath.winding(point))
}


// TODO replace naive implementation with a real one
fn cut_paths(paths: &Vec<Path>) -> Vec<Path> {
    if paths.is_empty() { return vec![] }

    let mut working_paths = Vec::<Path>::new();

    for (path_index, path) in paths.iter().enumerate() {
        if path_index == 0 || !path.source.can_cut() {
            working_paths.push(path.clone());
            continue;
        }

        let mut new_working_paths = Vec::<Path>::new();

        for working_path in working_paths.iter() {
            let intersections = get_path_intersections(&working_path, path, 0.25);

            if intersections.is_empty() {
                new_working_paths.push(working_path.clone());
            } else {
                let working_path_parts = cut_path(&working_path, &intersections)
                    .into_iter()
                    .filter(|part| !is_path_covered(part, path))
                    .collect::<Vec<Path>>();

                new_working_paths.extend(working_path_parts);

            }
        }

        new_working_paths.push(path.clone());

        working_paths = new_working_paths;
    }

    working_paths

}