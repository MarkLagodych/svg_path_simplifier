use crate::{
    Error,
    GenerateArgs,
    RenderArgs,

    svgcom::*
};

use std::{
    rc::Rc,
    cell::Ref,

    collections::HashMap,
    
    fs::File,
    path::PathBuf,
    io::prelude::*,
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
pub type PathIntersections = HashMap<usize, Vec<kurbo::LineIntersection>>;


/// Supports checking whether a point is inside
pub struct CoveringShape {
    pub source: SvgPathNode,
    pub bezpath: kurbo::BezPath
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
    let svg_path_nodes = get_svg_path_nodes(&svg, &args);

    let mut svgcom = SvgCom::new(svg.size.width(), svg.size.height());

    if !args.autocut {

        svgcom.read_from_svg_paths(&svg_path_nodes);
        
    } else {

        let paths = svg_path_nodes.iter()
            .map(|node| Path::from(&node))
            .collect::<Vec<Path>>();

        let paths = autocut_paths(&paths, args.precision);

        svgcom.read_from_paths(&paths);

    }

    write!(output, "{}", svgcom.to_string());

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


fn get_svg_path_nodes(svg: &usvg::Tree, args: &GenerateArgs) -> Vec<SvgPathNode> {
    svg.root.descendants()
        .filter(|node| 
            match *node.borrow() {
                usvg::NodeKind::Path(_) => true,
                _ => false
            })
        .map(|node| SvgPathNode::new(&node))
        .filter(|path_result| path_result.is_some())
        .map(|result| result.unwrap())
        .filter(|path| !args.onlystroked || path.get_svg_path().stroke.is_some())
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


    pub fn get_svg_path(&self) -> Ref<'_, usvg::Path> {
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


    fn is_closed(&self) -> bool {
        let path = self.get_svg_path();

        return path.data.commands().len() > 0
            && *path.data.commands().last().unwrap() == usvg::PathCommand::ClosePath
    }


    pub fn can_cover(&self) -> bool {
        self.is_closed() && self.get_svg_path().fill.is_some()
    }


    /// Test for checking whether a given point lies inside a shape
    pub fn test_winding(&self, winding: i32) -> bool {
        let path = self.get_svg_path();

        if let Some(fill) = &path.fill {
            match fill.rule {
                usvg::FillRule::EvenOdd => return winding % 2 != 0,
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


    pub fn is_covered_by(&self, shape: &CoveringShape) -> bool {
        use kurbo::ParamCurve;

        if self.source == shape.source {
            return false;
        }

        if self.segments.len() == 0 {
            return false;
        }

        // XXX Is this Ok?
        shape.covers_point(self.segments[self.segments.len()/2].eval(0.5))
    }
}



impl SvgPathPoints {
    pub fn from(svg_node: &SvgPathNode) -> Self {
        
        let path = svg_node.get_svg_path();
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
        
        let path = svg_node.get_svg_path();
        let path_data = Rc::clone(&path.data);

        Self {
            path_data,
            command_index: 0,
        }
    }
}


impl CoveringShape {
    pub fn new(path: &Path) -> Option<Self> {
        if !path.source.can_cover() || path.segments.len() == 0 {
            return None;
        }

        let mut bezpath = kurbo::BezPath::from_path_segments(path.segments.clone().into_iter());

        Some(Self {
            source: path.source.clone(),
            bezpath
        })
    }


    pub fn covers_point(&self, point: kurbo::Point) -> bool {
        use kurbo::Shape;

        let winding = self.bezpath.winding(point);
        self.source.test_winding(winding)
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


/// Checks whether two bounding boxes intersect
fn bbox_intersect(bbox1: kurbo::Rect, bbox2: kurbo::Rect) -> bool {
    let intersection = bbox1.intersect(bbox2);

    intersection.width() > 0. && intersection.height() > 0.
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


fn get_reverse_line_intersection(
    intersected: &kurbo::Line,
    intersecting: &kurbo::CubicBez,
    precision: f64
) -> Vec::<kurbo::LineIntersection> {
    use kurbo::Shape;

    return kurbo::PathSeg::Cubic(intersecting.clone())
        .intersect_line(intersected.clone())
        .iter()
        .map(|intersection| kurbo::LineIntersection {
            line_t: intersection.segment_t,
            segment_t: intersection.line_t
        })
        .collect::<Vec<kurbo::LineIntersection>>()
}


fn get_curve_intersection(
    intersected: &kurbo::PathSeg,
    intersecting: &kurbo::CubicBez,
    precision: f64
) -> Vec::<kurbo::LineIntersection> {
    use kurbo::Shape;

    if let kurbo::PathSeg::Line(line) = intersected {
        return get_reverse_line_intersection(line, intersecting, precision);
    }

    // XXX Is this efficient?
    if !bbox_intersect(intersected.bounding_box(), intersecting.bounding_box()) {
        return vec![]
    }

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


/// Requires intersections to be sorted by segment_t
fn cut_segment(
    segment: &kurbo::PathSeg,
    intersections: &Vec<kurbo::LineIntersection>
) -> Vec::<kurbo::PathSeg> {

    use kurbo::ParamCurve;

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
    intersections: &mut PathIntersections,
    precision: f64,
) {

    for (segment_index, intersected_segment) in intersected.segments.iter().enumerate() {
        for intersecting_segment in intersecting.segments.iter() {
            let segment_intersections = get_segment_intersection(
                intersected_segment,
                intersecting_segment,
                precision
            );

            if segment_intersections.is_empty() {
                continue;
            }

            if intersections.contains_key(&segment_index) {
                intersections.get_mut(&segment_index).unwrap()
                    .extend(segment_intersections);
            } else {
                intersections.insert(segment_index, segment_intersections);
            }
        }
    }
}


fn cut_path(path: &Path, intersections: &PathIntersections) -> Vec<Path> {

    let mut subpaths = Vec::<Path>::new();

    let new_path = || Path::new(&path.source);

    subpaths.push(new_path());

    for (index, segment) in path.segments.iter().enumerate() {
        if !intersections.contains_key(&index) {
            // A non-intersected segment is added to the last path as is
            subpaths.last_mut().unwrap().segments.push(segment.clone());
        } else {
            // A segment intersection ends the old path and starts a new one.
            // The subsegment before the intersection goes to the old path.
            // The subsegment after the intersection does to the new path.

            let mut subsegments = cut_segment(segment, &intersections[&index]).into_iter();

            if let Some(segment) = subsegments.next() {
                subpaths.last_mut().unwrap().segments.push(segment.clone());
            }

            // A segment may be intersected multiple times.
            // In that case, every intersection starting with the second one begins a new path
            for segment in subsegments {
                subpaths.push(new_path());
                subpaths.last_mut().unwrap().segments.push(segment.clone());
            }
        }
    }

    subpaths

}



fn autocut_paths(paths: &Vec<Path>, precision: f64) -> Vec<Path> {
    let intersections = intersect_paths(paths, precision);

    let cut_paths = cut_paths(paths, intersections);

    let covering_shapes = create_covering_shapes(paths);

    remove_covered_paths(cut_paths, &covering_shapes)
}



/// result[i] contains intersections of paths[i]
fn intersect_paths(paths: &Vec<Path>, precision: f64) -> Vec::<PathIntersections> {
    let mut path_intersections = Vec::<PathIntersections>::with_capacity(paths.len());

    for (index, intersected_path) in paths.iter().enumerate() {
        path_intersections.push(PathIntersections::new());

        for intersecting_path in &paths[index+1..] {
            get_path_intersections(intersected_path, intersecting_path, path_intersections.last_mut().unwrap(), precision);
        }
    }

    for intersections in path_intersections.iter_mut() {
        for (segment_index, segment_intersections) in intersections.iter_mut() {
            segment_intersections.sort_by(|a, b|
                a.segment_t.partial_cmp(&b.segment_t).unwrap()
            );
        } 
    }

    path_intersections
}


fn cut_paths(paths: &Vec<Path>, intersections: Vec<PathIntersections>) -> Vec<Path> {
    let mut cut_paths = Vec::<Path>::new();

    for (index, path) in paths.iter().enumerate() {
        if intersections[index].is_empty() {
            cut_paths.push(path.clone());
            continue;
        }

        cut_paths.extend(cut_path(path, &intersections[index]));
    }

    cut_paths
}


fn create_covering_shapes(paths: &Vec<Path>) -> Vec<Option<CoveringShape>> {
    let mut covering_shapes = Vec::<Option<CoveringShape>>::with_capacity(paths.len());

    for (index, path) in paths.iter().enumerate() {
        covering_shapes.push(CoveringShape::new(path));
    }

    covering_shapes
}


/// Requires path's sources to be in the same order as coveringshapes's sources
fn remove_covered_paths(paths: Vec<Path>, covering_shapes: &Vec<Option<CoveringShape>>) -> Vec<Path> {
    if paths.len() == 0 { return vec![] }

    let mut z_index = 0; /// The least index of shapes that can cover the paths left
    let mut last_svg_node = paths[0].source.clone();

    paths.into_iter()
        .enumerate()
        .filter(|(index, path)| {

            if last_svg_node != path.source {
                last_svg_node = path.source.clone();
                z_index += 1;
            }

            for shape in &covering_shapes[z_index+1..] {
                if shape.is_none() {
                    continue;
                }

                if path.is_covered_by(shape.as_ref().unwrap()) {
                    return false;
                }
            }

            true
        })
        .map(|(index, shape)| shape)
        .collect::<Vec::<Path>>()
}