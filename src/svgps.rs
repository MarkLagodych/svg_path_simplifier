use crate::{
    Error,
    GenerateArgs,
    RenderArgs,

    svgcom::*
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
pub struct SvgPath(usvg::Node);


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
    let svg_paths = get_svg_paths(&svg, &args);

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


fn get_svg_paths(svg: &usvg::Tree, args: &GenerateArgs) -> Vec<SvgPath> {
    svg.root.descendants()
        .filter(|node| 
            match *node.borrow() {
                usvg::NodeKind::Path(_) => true,
                _ => false
            })
        .map(|node| SvgPath::new(&node))
        .filter(|path_result| path_result.is_some())
        .map(|result| result.unwrap())
        .filter(|path| !args.onlystroked || path.borrow().stroke.is_some())
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