use image::io::Reader as ImageReader;
use gltf::json;
use voxelify::{create_glb, create_gltf_root, image_to_vertices, to_padded_byte_vector, Vertex};
use std::io::Write;
use std::path::Path;

fn main() {
    let image_path = "iron_sword.png";
    let output_path = "output.gltf";

    let img = load_image(image_path);
    let flipped = img.flipv().fliph();
    let vertices = image_to_vertices(&flipped, 2.0);
    let root = create_gltf_root(&vertices, None);
    let glb = create_glb(&root, &vertices).unwrap();

    let writer = std::fs::File::create("test.glb").expect("I/O error");
    glb.to_writer(writer).expect("glTF binary output error");
}

fn load_image(file_path: &str) -> image::DynamicImage {
    ImageReader::open(file_path).unwrap().decode().unwrap()
}

fn export_to_gltf(root: &gltf::json::Root, vertices: &[Vertex], output_dir: &Path) {
    let _ = std::fs::create_dir(output_dir);

    let writer = std::fs::File::create(output_dir.join("base.gltf")).expect("I/O error");
    json::serialize::to_writer_pretty(writer, &root).expect("Serialization error");

    let bin = to_padded_byte_vector(vertices.to_vec());
    let mut writer = std::fs::File::create(output_dir.join("buffer0.bin")).expect("I/O error");
    writer.write_all(&bin).expect("I/O error");
}
