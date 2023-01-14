#![allow(unused)]

extern crate usvg;
extern crate clap;
extern crate kurbo;


mod svgps;
mod svgcom;

use svgps::{
    generate_from_svg, render_to_svg
};


use std::{
    fs::File,
    path::PathBuf,
};


/// SVG Path Simplifier
/// This program simplifies SVG and emits LineTo/MoveTo/BezierTo commands.
/// The purpose is to help plotters understand SVG in the same way that people do.
#[derive(clap::Parser)]
#[command(version, about, long_about, verbatim_doc_comment)]
struct Args {
    #[command(subcommand)]
    command: ArgCommand
}


#[derive(clap::Subcommand)]
enum ArgCommand {
    /// Simplify SVG image and generate SVG command file (*.svgcom)
    Generate(GenerateArgs),

    /// Render svgcom file (useful for previewing before submitting to plotters)
    Render(RenderArgs)
}


#[derive(clap::Args)]
pub struct GenerateArgs {
    /// SVG image file (.svg)
    input: PathBuf,

    /// SVG commands file (.svgcom)
    output: PathBuf,

    /// Automatically cut the path segments that are covered and therefore invisible
    #[arg(short = 'c', long)]
    autocut: bool,

    /// Remove the paths that are too short (shorter than PRECISION) after autocut
    #[arg(short = 'r', long)]
    polish: bool,

    /// Precision of autocut/polish commands (in pixels)
    #[arg(short = 'p', long, default_value_t = 0.5)]
    precision: f64,

    // /// Do not convert ClosePath ("Z") commands
    // #[arg(short = 'z', long)]
    // noclose: bool,

    /// Convert only stroked paths
    #[arg(short = 's', long)]
    onlystroked: bool,
}


#[derive(clap::Args)]
pub struct RenderArgs {
    /// SVG commands file (.svgcom)
    input: PathBuf,

    /// SVG image file (.svg)
    output: PathBuf,

    /// SVG stroke attribute for the generated path
    #[arg(short = 's', long, default_value = "#000000")]
    stroke: String,

    /// SVG stroke-width attribute for the generated path
    #[arg(id = "WIDTH", short = 'w', long = "stroke-width", default_value_t = 1.0)]
    stroke_width: f64,
}


pub type Error = String;


fn main() {
    use clap::Parser;

    let args = Args::parse();

    let result: Result<(), Error> = match args.command {
        ArgCommand::Generate(args) => generate_from_svg(args),
        ArgCommand::Render(args) => render_to_svg(args),
    };

    if let Err(message) = result {
        println!("Error: {}", message);
    }
}
