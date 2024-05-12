use clap::{Parser, ValueEnum};
use image::io::Reader as ImageReader;
use std::path::PathBuf;
use voxelify::{create_glb, create_gltf_root, image_to_vertices};

#[derive(Debug, Default, PartialEq, Clone, ValueEnum)]
enum Format {
    #[default]
    Glb,
    Gltf,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    format: Format,
    #[arg(short, long)]
    output: PathBuf,
    #[arg(short, long)]
    vertical_flip: bool,
    #[arg(short, long)]
    horizontal_flip: bool,
    #[arg(short, long, default_value_t = 2.0)]
    z_height: f32,
    #[arg(short, long)]
    uri: Option<String>,
}

fn main() {
    let args = Args::parse();

    let img = {
        let mut img = load_image(args.input.as_path().to_str().unwrap());

        if args.vertical_flip {
            img = img.flipv();
        }
        if args.horizontal_flip {
            img = img.fliph();
        }

        img
    };

    let vertices = image_to_vertices(&img, args.z_height);
    let root = create_gltf_root(&vertices, args.uri);
    let glb = create_glb(&root, &vertices).unwrap();

    match args.format {
        Format::Glb => {
            let writer = std::fs::File::create(args.output).expect("I/O error");
            glb.to_writer(writer).expect("glTF binary output error");
        }
        Format::Gltf => {
            panic!("GLTF output is not implemented yet");
        }
    }
}

fn load_image(file_path: &str) -> image::DynamicImage {
    ImageReader::open(file_path).unwrap().decode().unwrap()
}
