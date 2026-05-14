use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::Image,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_pmx::prelude::*;
use encoding_rs::SHIFT_JIS;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{ZipWriter, write::SimpleFileOptions};

fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{unique}{extension}"))
}

fn tiny_png_bytes() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0x0F, 0x04, 0x00, 0x09, 0xFB, 0x03, 0xFD, 0xFB, 0x5E, 0x6B, 0x2B, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_text(bytes: &mut Vec<u8>, value: &str) {
    push_i32(bytes, value.len() as i32);
    bytes.extend_from_slice(value.as_bytes());
}

fn push_vec2(bytes: &mut Vec<u8>, value: [f32; 2]) {
    push_f32(bytes, value[0]);
    push_f32(bytes, value[1]);
}

fn push_vec3(bytes: &mut Vec<u8>, value: [f32; 3]) {
    push_f32(bytes, value[0]);
    push_f32(bytes, value[1]);
    push_f32(bytes, value[2]);
}

fn push_vec4(bytes: &mut Vec<u8>, value: [f32; 4]) {
    push_f32(bytes, value[0]);
    push_f32(bytes, value[1]);
    push_f32(bytes, value[2]);
    push_f32(bytes, value[3]);
}

fn push_vertex(bytes: &mut Vec<u8>, position: [f32; 3], uv: [f32; 2]) {
    push_vec3(bytes, position);
    push_vec3(bytes, [0.0, 0.0, 1.0]);
    push_vec2(bytes, uv);
    push_u8(bytes, 0);
    push_i32(bytes, -1);
    push_f32(bytes, 1.0);
}

fn push_material(bytes: &mut Vec<u8>, texture_index: i32) {
    push_text(bytes, "material");
    push_text(bytes, "material");
    push_vec4(bytes, [1.0, 1.0, 1.0, 1.0]);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_f32(bytes, 1.0);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_u8(bytes, 0);
    push_vec4(bytes, [0.0, 0.0, 0.0, 0.0]);
    push_f32(bytes, 1.0);
    push_i32(bytes, texture_index);
    push_i32(bytes, -1);
    push_u8(bytes, 0);
    push_u8(bytes, 0);
    push_i32(bytes, -1);
    push_text(bytes, "");
    push_i32(bytes, 3);
}

fn build_minimal_pmx_bytes(texture_paths: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PMX ");
    push_f32(&mut bytes, 2.0);
    push_u8(&mut bytes, 8);
    push_u8(&mut bytes, 1);
    push_u8(&mut bytes, 0);
    bytes.extend_from_slice(&[4, 4, 4, 4, 4, 4]);
    push_text(&mut bytes, "zip support sample");
    push_text(&mut bytes, "zip support sample");
    push_text(&mut bytes, "");
    push_text(&mut bytes, "");

    push_i32(&mut bytes, 3);
    push_vertex(&mut bytes, [0.0, 0.0, 0.0], [0.0, 0.0]);
    push_vertex(&mut bytes, [1.0, 0.0, 0.0], [1.0, 0.0]);
    push_vertex(&mut bytes, [0.0, 1.0, 0.0], [0.0, 1.0]);

    push_i32(&mut bytes, 3);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 2);

    push_i32(&mut bytes, texture_paths.len() as i32);
    for path in texture_paths {
        push_text(&mut bytes, path);
    }

    push_i32(&mut bytes, 1);
    push_material(&mut bytes, if texture_paths.is_empty() { -1 } else { 0 });

    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);

    bytes
}

fn write_zip(entries: Vec<(&str, &[u8])>) -> PathBuf {
    let zip_path = unique_temp_path("bevy_pmx_zip_support", ".zip");
    let file = fs::File::create(&zip_path).expect("should create zip fixture");
    let mut writer = ZipWriter::new(file);

    for (name, bytes) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .expect("should add zip entry");
        writer.write_all(bytes).expect("should write zip entry");
    }

    writer.finish().expect("should finish zip archive");
    zip_path
}

fn import_model(source: &PmxSource, model_entry: &str) -> Pmx {
    let model_location = source.resolve(model_entry);
    let document = PmxDocument::from_bytes(
        &source
            .read_bytes(&model_location)
            .expect("should read the PMX document from the source"),
    )
    .expect("should parse the PMX document");

    import_pmx(
        document,
        &PmxImportContext {
            source: Some(source.clone()),
            ..PmxImportContext::default()
        },
    )
    .model
}

fn placeholder_texture_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 0, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn decode_texture_or_placeholder(location: &PmxSourceLocation, bytes: &[u8]) -> Image {
    let Some(extension) = location
        .extension()
        .map(|extension| extension.to_ascii_lowercase())
    else {
        return placeholder_texture_image();
    };

    let image_type = ImageType::Extension(extension.as_str());
    Image::from_buffer(
        bytes,
        image_type,
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .unwrap_or_else(|_| placeholder_texture_image())
}

fn load_texture_or_placeholder(source: &PmxSource, texture: &PmxResolvedPath) -> Image {
    match source.read_bytes(texture.location()) {
        Ok(bytes) => decode_texture_or_placeholder(texture.location(), &bytes),
        Err(_) => placeholder_texture_image(),
    }
}

fn is_placeholder_texture(image: &Image) -> bool {
    image.texture_descriptor.size
        == Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        }
        && image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb
        && image.data.as_deref() == Some(&[255, 0, 255, 255])
}

#[test]
fn zip_name_encoding_auto_decodes_shift_jis_names() {
    let (raw, _, _) = SHIFT_JIS.encode("Texture/\u{65E9}.png");

    assert_eq!(
        ZipNameEncoding::Auto.decode_name(raw.as_ref()).as_deref(),
        Some("Texture/\u{65E9}.png")
    );
}

#[test]
fn explicit_zip_name_encoding_does_not_guess_other_encodings() {
    let (raw, _, _) = SHIFT_JIS.encode("Texture/\u{65E9}.png");

    assert!(ZipNameEncoding::Utf8.decode_name(raw.as_ref()).is_none());
}

#[test]
fn zip_archive_pmx_and_textures_round_trip_through_the_source_abstraction() {
    let texture_bytes = tiny_png_bytes();
    let pmx_bytes = build_minimal_pmx_bytes(&["texture/valid.png"]);
    let zip_path = write_zip(vec![
        ("Model/model.pmx", pmx_bytes.as_slice()),
        ("Model/Texture/Valid.PNG", texture_bytes),
    ]);
    let source = PmxSource::zip_with_encoding(zip_path.clone(), "Model", ZipNameEncoding::Auto);
    let model = import_model(&source, "model.pmx");

    assert_eq!(model.texture_paths.len(), 1);
    let texture = &model.texture_paths[0];
    assert_eq!(
        texture.resolved_path(),
        Path::new("Model/Texture/Valid.PNG")
    );
    assert_eq!(
        texture.location(),
        &PmxSourceLocation::zip(zip_path.clone(), "Model/Texture/Valid.PNG")
    );
    assert!(source.contains_location(texture.location()));

    let bytes = source
        .read_bytes(texture.location())
        .expect("should read texture bytes from the zip archive");
    let image = decode_texture_or_placeholder(texture.location(), &bytes);

    assert_eq!(image.texture_descriptor.size.width, 1);
    assert_eq!(image.texture_descriptor.size.height, 1);
    assert!(!is_placeholder_texture(&image));
}

#[test]
fn folder_source_regression_still_loads_texture_bytes_from_disk() {
    let temp_root = unique_temp_path("bevy_pmx_folder_source", "");
    let model_root = temp_root.join("model");
    let texture_root = model_root.join("Texture");
    fs::create_dir_all(&texture_root).expect("should create texture directory");
    let texture_bytes = tiny_png_bytes();
    fs::write(texture_root.join("Face.PNG"), texture_bytes).expect("should create texture file");
    let pmx_path = model_root.join("model.pmx");
    let pmx_bytes = build_minimal_pmx_bytes(&["Texture/Face.PNG"]);
    fs::write(&pmx_path, pmx_bytes).expect("should create PMX file");

    let source = PmxSource::folder(&model_root);
    let model = import_model(&source, "model.pmx");
    let expected_texture_path = texture_root.join("Face.PNG");

    assert_eq!(model.texture_paths.len(), 1);
    let texture = &model.texture_paths[0];
    assert_eq!(
        texture.location(),
        &PmxSourceLocation::disk(expected_texture_path)
    );
    assert!(source.contains_location(texture.location()));
    assert_eq!(
        fs::canonicalize(
            texture
                .location()
                .as_disk_path()
                .expect("disk texture path")
        )
        .expect("resolved path should canonicalize"),
        fs::canonicalize(texture_root.join("Face.PNG"))
            .expect("expected texture path should canonicalize")
    );

    let bytes = source
        .read_bytes(texture.location())
        .expect("should read texture bytes from the folder source");
    let image = decode_texture_or_placeholder(texture.location(), &bytes);

    assert_eq!(image.texture_descriptor.size.width, 1);
    assert_eq!(image.texture_descriptor.size.height, 1);
    assert!(!is_placeholder_texture(&image));
}

#[test]
fn missing_or_bad_zip_resources_fall_back_to_placeholder_images() {
    let pmx_bytes = build_minimal_pmx_bytes(&[
        "texture/valid.png",
        "texture/bad.png",
        "texture/missing.png",
    ]);
    let texture_bytes = tiny_png_bytes();
    let zip_path = write_zip(vec![
        ("Model/model.pmx", pmx_bytes.as_slice()),
        ("Model/Texture/Valid.PNG", texture_bytes),
        ("Model/Texture/Bad.PNG", b"not an image"),
    ]);
    let source = PmxSource::zip_with_encoding(zip_path, "Model", ZipNameEncoding::Auto);
    let model = import_model(&source, "model.pmx");

    assert_eq!(model.texture_paths.len(), 3);
    let images: Vec<Image> = model
        .texture_paths
        .iter()
        .map(|texture| load_texture_or_placeholder(&source, texture))
        .collect();

    assert!(!is_placeholder_texture(&images[0]));
    assert!(is_placeholder_texture(&images[1]));
    assert!(is_placeholder_texture(&images[2]));
}
