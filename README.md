# Voxelify

Convert a 2D pixel art image into GLTF 3D object using voxels and face-culling

[![Crates.io][crates-badge]][crates-url]https://crates.io/crates/voxelify
[![MIT licensed][mit-badge]][mit-url]https://github.com/EngoDev/voxelify/blob/main/LICENSE-MIT
[![License][apache-badge]][apache-url]https://github.com/EngoDev/voxelify/blob/main/LICENSE-APACHE

# Example


| 2D Image               | 3D GLB                 |
| ------------------------ | ------------------------ |
| ![](assets/smiley.png) | ![](assets/smiley.gif) |


# GLTF support

Currently, the crate supports only GLB in GLTF 2.0 specs.



Support for GLTF ascii is planned and can be tracked here: https://github.com/EngoDev/voxelify/issues/1

# Usage

```bash
cargo run --release -- --help
```

Or if you want to use the functions in your code you can do:

```bash
cargo add voxelify
```

[crates-badge]: https://img.shields.io/crates/v/voxelify.svg
[crates-url]: https://crates.io/crates/voxelify
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/EngoDev/voxelify/blob/main/LICENSE-MIT
[apache-badge]: https://img.shields.io/badge/License-Apache_2.0-blue.svg
[apache-url]: https://github.com/EngoDev/voxelify/blob/main/LICENSE-APACHE
