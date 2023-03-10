use crate::{
    Error,
    svgps::{SvgPathNode, SvgPathPoints, Path},
};


pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}


/// Final .svgcom representation
pub struct SvgCom {
    pub view_size: ImageSize,
    pub commands: kurbo::BezPath,
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


    pub fn read_from_paths(&mut self, paths: &Vec<Path>) {
        for path in paths {
            self.read_from_path(path);
        }
    }


    fn read_from_path(&mut self, path: &Path) {
        self.commands.extend(kurbo::BezPath::from_path_segments(path.segments.clone().into_iter()));
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

        Self::validate_svgcom_metrics(&metrics, &commands, &coords)?;

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

    fn validate_svgcom_metrics(metrics: &Vec<u32>, commands: &Vec<char>, coords: &Vec<f64>) -> Result<(), Error> {
        if commands.len() != metrics[2] as usize || coords.len() != metrics[3] as usize {
            return Err("Data length does not match the header information".to_string());
        }

        Ok(())
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
                _ => panic!("unexpected command"),
            };
        }

        npoints
    }


    pub fn coordinates_count(&self) -> usize {
        self.points_count() * 2
    }


    pub fn to_svg_path_data_str(&self) -> String {
        self.commands.to_svg()
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


impl std::fmt::Display for SvgCom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format_svgcom(f)
    }
}