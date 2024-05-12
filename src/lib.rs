use gltf::json::validation::USize64;
use gltf::json::{self, Index};
use gltf::json::{buffer, validation::Checked::Valid};
use image::{GenericImageView, Pixel};
use nalgebra::Vector2;
use std::borrow::Cow;

use self::error::ExtrudeImageError;

mod error;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

pub fn create_glb<'a>(
    root: &gltf::json::Root,
    vertices: &[Vertex],
) -> Result<gltf::binary::Glb<'a>, ExtrudeImageError> {
    let json_string = json::serialize::to_string(&root)?; //.expect("Serialization error");
    let mut json_offset = json_string.len();
    align_to_multiple_of_four(&mut json_offset);
    let glb = gltf::binary::Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            // N.B., the size of binary glTF file is limited to range of `u32`.
            length: (json_offset + calculate_buffer_length(vertices)).try_into()?,
        },
        bin: Some(Cow::Owned(to_padded_byte_vector(vertices.to_vec()))),
        json: Cow::Owned(json_string.into_bytes()),
    };

    Ok(glb)
}

pub fn create_gltf_root(vertices: &[Vertex], uri: Option<String>) -> gltf::json::Root {
    let mut root = gltf::json::Root::default();

    let vertex_buffer_length = calculate_buffer_length(vertices);

    let buffer = root.push(gltf::json::Buffer {
        byte_length: USize64::from(vertex_buffer_length),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri,
    });

    let vertex_buffer_view = root.push(buffer::View {
        buffer,
        byte_length: USize64::from(vertex_buffer_length),
        byte_offset: None,
        byte_stride: Some(buffer::Stride(std::mem::size_of::<Vertex>())),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(buffer::Target::ArrayBuffer)),
    });

    let VoxelAccessors {
        positions,
        normals,
        colors,
    } = create_accessors(&mut root, &vertex_buffer_view, vertices);

    let primitive = json::mesh::Primitive {
        attributes: {
            let mut map = std::collections::BTreeMap::new();
            map.insert(Valid(json::mesh::Semantic::Positions), positions);
            map.insert(Valid(json::mesh::Semantic::Normals), normals);
            map.insert(Valid(json::mesh::Semantic::Colors(0)), colors);
            map
        },
        extensions: Default::default(),
        extras: Default::default(),
        indices: None,
        material: None,
        mode: Valid(json::mesh::Mode::Triangles),
        targets: None,
    };

    let mesh = root.push(json::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives: vec![primitive],
        weights: None,
    });

    let node = root.push(json::Node {
        mesh: Some(mesh),
        ..Default::default()
    });

    root.push(json::Scene {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        nodes: vec![node],
    });

    root
}

/// Converts an image to list of vertices that can be used to create a mesh
/// Each pixel will be converted to a cube with height determined by the height argument
pub fn image_to_vertices(image: &image::DynamicImage, height: f32) -> Vec<Vertex> {
    let (image_width, image_height) = image.dimensions();
    let mut vertices = Vec::new();

    for y in 0..image_height {
        for x in 0..image_width {
            let pixel = image.get_pixel(x, y).to_rgb();
            if pixel == image::Rgb([0, 0, 0]) {
                continue;
            }

            let faces = cull_faces(image, Vector2::new(x, y));

            for face in faces {
                vertices.extend(create_pixel_verticies_face(
                    [x as f32, y as f32],
                    pixel,
                    height,
                    &face,
                ));
            }
            // vertices.extend(create_pixel_verticies([x as f32, y as f32], pixel, height));
        }
    }
    vertices
}

/// Struct for storing accessors for the GLTF vertex buffer
struct VoxelAccessors {
    positions: Index<json::Accessor>,
    normals: Index<json::Accessor>,
    colors: Index<json::Accessor>,
}

// Enum for the faces of a voxel
// Used for culling faces that are not visible
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum Face {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Back,
}

/// Returns the faces that should be visible for a given pixel
fn cull_faces(
    image: &image::DynamicImage,
    pos: Vector2<u32>,
) -> Vec<Face> {

    let (image_width, image_height) = image.dimensions();

    // Initialize with Up and Down faces as they're always visible in a 2D image
    let mut faces = vec![Face::Up, Face::Down];

    match pos.x {
        0 => faces.push(Face::Left),
        x if x == image_width - 1 => faces.push(Face::Right),
        _ => {
            // Check on the x-axis for adjacent pixels that are empty.
            if is_empty_pixel(image.get_pixel(pos.x + 1, pos.y).to_rgb()) {
                faces.push(Face::Right);
            }
            if is_empty_pixel(image.get_pixel(pos.x - 1, pos.y).to_rgb()) {
                faces.push(Face::Left);
            }
        }
    }

    match pos.y {
        0 => faces.push(Face::Forward),
        y if y == image_height - 1 => faces.push(Face::Back),
        _ => {
            // Check on the y-axis for adjacent pixels that are empty.
            if is_empty_pixel(image.get_pixel(pos.x, pos.y + 1).to_rgb()) {
                faces.push(Face::Back);
            }
            if is_empty_pixel(image.get_pixel(pos.x, pos.y - 1).to_rgb()) {
                faces.push(Face::Forward);
            }
        }
    }

    faces
}


fn create_accessors(
    root: &mut gltf::json::Root,
    buffer_view: &Index<buffer::View>,
    vertices: &[Vertex],
) -> VoxelAccessors {
    let (min, max) = bounding_coords(vertices);

    let positions = root.push(json::Accessor {
        buffer_view: Some(*buffer_view),
        byte_offset: Some(USize64(0)),
        count: USize64::from(vertices.len()),
        component_type: Valid(json::accessor::GenericComponentType(
            json::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(json::accessor::Type::Vec3),
        min: Some(json::Value::from(Vec::from(min))),
        max: Some(json::Value::from(Vec::from(max))),
        name: None,
        normalized: false,
        sparse: None,
    });

    let normals = root.push(json::Accessor {
        buffer_view: Some(*buffer_view),
        byte_offset: Some(USize64::from(3 * std::mem::size_of::<f32>())),
        count: USize64::from(vertices.len()),
        component_type: Valid(json::accessor::GenericComponentType(
            json::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(json::accessor::Type::Vec3),
        min: Some(json::Value::from(Vec::from([-1.0, -1.0, -1.0]))),
        max: Some(json::Value::from(Vec::from([1.0, 1.0, 1.0]))),
        name: None,
        normalized: false,
        sparse: None,
    });

    let colors = root.push(json::Accessor {
        buffer_view: Some(*buffer_view),
        byte_offset: Some(USize64::from(6 * std::mem::size_of::<f32>())),
        count: USize64::from(vertices.len()),
        component_type: Valid(json::accessor::GenericComponentType(
            json::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(json::accessor::Type::Vec3),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    });

    VoxelAccessors {
        positions,
        normals,
        colors,
    }
}

fn is_empty_pixel(pixel: image::Rgb<u8>) 
-> bool {
    pixel == image::Rgb([0, 0, 0])
}

#[rustfmt::skip]
fn create_pixel_verticies_face(
    pos: [f32; 2],
    color: image::Rgb<u8>,
    height: f32,
    face: &Face,
) -> Vec<Vertex> {
    let color = [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    ];

    match face {
        // Top face (z = 1)
        Face::Up => {
            vec![
                Vertex { position: [pos[0], pos[1], height], normal: [0.0, 0.0, 1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], height], normal: [0.0, 0.0, 1.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, height], normal: [0.0, 0.0, 1.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, height], normal: [0.0, 0.0, 1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], height], normal: [0.0, 0.0, 1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, height], normal: [0.0, 0.0, 1.0], color },
            ]
        }
        Face::Down => {
            vec![
                Vertex { position: [pos[0], pos[1], 0.0], normal: [0.0, 0.0, -1.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, 0.0], normal: [0.0, 0.0, -1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], 0.0], normal: [0.0, 0.0, -1.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, 0.0], normal: [0.0, 0.0, -1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, 0.0], normal: [0.0, 0.0, -1.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], 0.0], normal: [0.0, 0.0, -1.0], color },
            ]
        }
        Face::Forward => {
            vec![
                Vertex { position: [pos[0], pos[1], 0.0], normal: [0.0, -1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], 0.0], normal: [0.0, -1.0, 0.0], color },
                Vertex { position: [pos[0], pos[1], height], normal: [0.0, -1.0, 0.0], color },
                Vertex { position: [pos[0], pos[1], height], normal: [0.0, -1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], 0.0], normal: [0.0, -1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], height], normal: [0.0, -1.0, 0.0], color },
            ]
        }
        Face::Back => {
            vec![
                Vertex { position: [pos[0], pos[1] + 1.0, 0.0], normal: [0.0, 1.0, 0.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, height], normal: [0.0, 1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, 0.0], normal: [0.0, 1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, 0.0], normal: [0.0, 1.0, 0.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, height], normal: [0.0, 1.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, height], normal: [0.0, 1.0, 0.0], color },
            ]
        }
        Face::Left => {
            vec![
                Vertex { position: [pos[0], pos[1], 0.0], normal: [-1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0], pos[1], height], normal: [-1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, 0.0], normal: [-1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, 0.0], normal: [-1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0], pos[1], height], normal: [-1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0], pos[1] + 1.0, height], normal: [-1.0, 0.0, 0.0], color },
            ]
        }
        Face::Right => {
            vec![
                Vertex { position: [pos[0] + 1.0, pos[1], 0.0], normal: [1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, 0.0], normal: [1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], height], normal: [1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, 0.0], normal: [1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1] + 1.0, height], normal: [1.0, 0.0, 0.0], color },
                Vertex { position: [pos[0] + 1.0, pos[1], height], normal: [1.0, 0.0, 0.0], color },
            ]
        }
    }
}

/// Takes a poistion of a pixel and it's color and converts it to a cube
/// You can specify the height of the cube using the height arguement
pub fn create_pixel_verticies(pos: [f32; 2], color: image::Rgb<u8>, height: f32) -> Vec<Vertex> {
    let color = [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    ];

    vec![
        // Top face (z = 1)
        Vertex {
            position: [pos[0], pos[1], height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, height],
            normal: [0.0, 0.0, 1.0],
            color,
        },
        // Bottom face (z = 0)
        Vertex {
            position: [pos[0], pos[1], 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], 0.0],
            normal: [0.0, 0.0, -1.0],
            color,
        },
        // Front face
        Vertex {
            position: [pos[0], pos[1], 0.0],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], 0.0],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1], height],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1], height],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], 0.0],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], height],
            normal: [0.0, -1.0, 0.0],
            color,
        },
        // Back face
        Vertex {
            position: [pos[0], pos[1] + 1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, height],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, height],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, height],
            normal: [0.0, 1.0, 0.0],
            color,
        },
        // Left face
        Vertex {
            position: [pos[0], pos[1], 0.0],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1], height],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1], height],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0], pos[1] + 1.0, height],
            normal: [-1.0, 0.0, 0.0],
            color,
        },
        // Right face
        Vertex {
            position: [pos[0] + 1.0, pos[1], 0.0],
            normal: [1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], height],
            normal: [1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1] + 1.0, height],
            normal: [1.0, 0.0, 0.0],
            color,
        },
        Vertex {
            position: [pos[0] + 1.0, pos[1], height],
            normal: [1.0, 0.0, 0.0],
            color,
        },
    ]
}

/// Calculate bounding coordinates of a list of vertices, used for the clipping distance of the model
fn bounding_coords(points: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    for point in points {
        let p = point.position;
        for i in 0..3 {
            min[i] = f32::min(min[i], p[i]);
            max[i] = f32::max(max[i], p[i]);
        }
    }
    (min, max)
}

fn align_to_multiple_of_four(n: &mut usize) {
    *n = (*n + 3) & !3;
}

pub fn to_padded_byte_vector<T>(vec: Vec<T>) -> Vec<u8> {
    let byte_length = vec.len() * std::mem::size_of::<T>();
    let byte_capacity = vec.capacity() * std::mem::size_of::<T>();
    let alloc = vec.into_boxed_slice();
    let ptr = Box::<[T]>::into_raw(alloc) as *mut u8;
    let mut new_vec = unsafe { Vec::from_raw_parts(ptr, byte_length, byte_capacity) };
    while new_vec.len() % 4 != 0 {
        new_vec.push(0); // pad to multiple of four bytes
    }
    new_vec
}

#[inline]
fn calculate_buffer_length(vertices: &[Vertex]) -> usize {
    std::mem::size_of_val(vertices)
}

pub fn generate_indices(vertex_count: usize) -> Vec<u32> {
    let indices_count = vertex_count / 4;
    let mut indices = Vec::<u32>::with_capacity(indices_count);
    (0..indices_count).for_each(|vert_index| {
        let vert_index = vert_index as u32 * 4u32;
        indices.push(vert_index);
        indices.push(vert_index + 1);
        indices.push(vert_index + 2);
        indices.push(vert_index);
        indices.push(vert_index + 2);
        indices.push(vert_index + 3);
    });
    indices
}
