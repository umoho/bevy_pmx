use bevy::{
    asset::RenderAssetUsages,
    asset::{AssetPlugin, AssetServer, LoadState},
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::{App, Assets, Image},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_pmx::prelude::*;
use encoding_rs::SHIFT_JIS;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use zip::{ZipWriter, write::SimpleFileOptions};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let process_id = std::process::id();

    loop {
        let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = temp_dir.join(format!("{prefix}_{process_id}_{unique}"));

        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("should create temp directory {}: {error}", path.display()),
        }
    }
}

fn unique_temp_file(prefix: &str, extension: &str) -> PathBuf {
    unique_temp_dir(prefix).join(format!("fixture{extension}"))
}

fn unique_asset_root(prefix: &str) -> PathBuf {
    let temp_dir = Path::new("assets");
    let process_id = std::process::id();
    let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    temp_dir.join(format!("{prefix}_{process_id}_{unique}"))
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

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
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

fn push_rigid_body(bytes: &mut Vec<u8>) {
    push_text(bytes, "rigid");
    push_text(bytes, "rigid");
    push_i32(bytes, -1);
    push_u8(bytes, 1);
    push_u16(bytes, 0xffff);
    push_u8(bytes, 0);
    push_vec3(bytes, [0.5, 0.5, 0.5]);
    push_vec3(bytes, [0.0, 1.0, 0.0]);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_f32(bytes, 1.0);
    push_f32(bytes, 0.5);
    push_f32(bytes, 0.5);
    push_f32(bytes, 0.3);
    push_f32(bytes, 0.4);
    push_u8(bytes, 1);
}

fn push_joint(bytes: &mut Vec<u8>) {
    push_text(bytes, "joint");
    push_text(bytes, "joint");
    push_u8(bytes, 0);
    push_i32(bytes, 0);
    push_i32(bytes, 0);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_vec3(bytes, [-1.0, -1.0, -1.0]);
    push_vec3(bytes, [1.0, 1.0, 1.0]);
    push_vec3(bytes, [-0.1, -0.1, -0.1]);
    push_vec3(bytes, [0.1, 0.1, 0.1]);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
    push_vec3(bytes, [0.0, 0.0, 0.0]);
}

fn push_soft_body_config(bytes: &mut Vec<u8>) {
    for value in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2] {
        push_f32(bytes, value);
    }
}

fn push_soft_body_cluster(bytes: &mut Vec<u8>) {
    for value in [1.3, 1.4, 1.5, 1.6, 1.7, 1.8] {
        push_f32(bytes, value);
    }
}

fn push_soft_body(bytes: &mut Vec<u8>) {
    push_text(bytes, "soft");
    push_text(bytes, "soft");
    push_u8(bytes, 0);
    push_i32(bytes, -1);
    push_u8(bytes, 1);
    push_u16(bytes, 0xffff);
    push_u8(bytes, 0);
    push_i32(bytes, 1);
    push_i32(bytes, 2);
    push_f32(bytes, 1.0);
    push_f32(bytes, 0.05);
    push_i32(bytes, 0);
    push_soft_body_config(bytes);
    push_soft_body_cluster(bytes);
    push_i32(bytes, 5);
    push_i32(bytes, 6);
    push_i32(bytes, 7);
    push_i32(bytes, 8);
    push_f32(bytes, 0.2);
    push_f32(bytes, 0.3);
    push_f32(bytes, 0.4);
    push_i32(bytes, 1);
    push_i32(bytes, 0);
    push_i32(bytes, 0);
    push_u8(bytes, 1);
    push_i32(bytes, 1);
    push_i32(bytes, 0);
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

fn build_physics_pmx_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PMX ");
    push_f32(&mut bytes, 2.1);
    push_u8(&mut bytes, 8);
    push_u8(&mut bytes, 1);
    push_u8(&mut bytes, 0);
    bytes.extend_from_slice(&[4, 4, 4, 4, 4, 4]);
    push_text(&mut bytes, "physics sample");
    push_text(&mut bytes, "physics sample");
    push_text(&mut bytes, "");
    push_text(&mut bytes, "");

    push_i32(&mut bytes, 1);
    push_vertex(&mut bytes, [0.0, 0.0, 0.0], [0.0, 0.0]);

    push_i32(&mut bytes, 0);

    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);

    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);

    push_i32(&mut bytes, 1);
    push_rigid_body(&mut bytes);

    push_i32(&mut bytes, 1);
    push_joint(&mut bytes);

    push_i32(&mut bytes, 1);
    push_soft_body(&mut bytes);

    bytes
}

fn write_zip(entries: Vec<(&str, &[u8])>) -> PathBuf {
    let zip_path = unique_temp_file("bevy_pmx_zip_support", ".zip");
    write_zip_at(&zip_path, entries);
    zip_path
}

fn write_zip_at(zip_path: &Path, entries: Vec<(&str, &[u8])>) {
    if let Some(parent) = zip_path.parent() {
        fs::create_dir_all(parent).expect("should create zip parent directory");
    }

    let file = fs::File::create(&zip_path).expect("should create zip fixture");
    let mut writer = ZipWriter::new(file);

    for (name, bytes) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .expect("should add zip entry");
        writer.write_all(bytes).expect("should write zip entry");
    }

    writer.finish().expect("should finish zip archive");
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("should create parent directory");
    }
    fs::write(path, bytes).expect("should write fixture bytes");
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

fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        bevy::MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default_nearest(),
        PmxPlugin::with_settings(PmxLoaderSettings {
            load_textures: true,
            load_materials: false,
            load_meshes: false,
            load_bones: false,
            load_morphs: false,
            load_physics: false,
            keep_raw_document: true,
            resolver: Default::default(),
            zip_name_encoding: ZipNameEncoding::Auto,
        }),
    ));
    app
}

fn build_physics_test_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        bevy::MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default_nearest(),
        PmxPlugin::with_settings(PmxLoaderSettings {
            load_textures: false,
            load_materials: false,
            load_meshes: false,
            load_bones: false,
            load_morphs: false,
            load_physics: true,
            keep_raw_document: true,
            resolver: Default::default(),
            zip_name_encoding: ZipNameEncoding::Auto,
        }),
    ));
    app
}

fn load_pmx_handle(app: &mut App, path: String) -> bevy::asset::Handle<Pmx> {
    let asset_server = app.world().resource::<AssetServer>().clone();
    asset_server.load(path)
}

fn wait_for_pmx_asset(app: &mut App, handle: &bevy::asset::Handle<Pmx>) {
    let asset_server = app.world().resource::<AssetServer>().clone();

    for _ in 0..64 {
        let load_state = asset_server.load_state(handle.id());
        if matches!(load_state, LoadState::Loaded)
            && asset_server.is_loaded_with_dependencies(handle.id())
        {
            return;
        }
        if let LoadState::Failed(error) = load_state {
            panic!("PMX asset failed to load: {error:?}");
        }
        app.update();
    }

    panic!(
        "PMX asset did not finish loading; final load state was {:?}",
        asset_server.load_state(handle.id())
    );
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
    let temp_root = unique_temp_dir("bevy_pmx_folder_source");
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

#[test]
fn asset_server_loads_zip_pmx_assets_from_archive_entries() {
    let asset_root = unique_asset_root("bevy_pmx_loader_zip");
    let zip_path = asset_root.join("character.zip");
    let load_path = asset_root
        .strip_prefix("assets")
        .expect("asset path should live under assets")
        .join("character.zip");
    let pmx_bytes = build_minimal_pmx_bytes(&["Texture/Face.PNG"]);

    write_zip_at(
        &zip_path,
        vec![
            ("Package/model.pmx", pmx_bytes.as_slice()),
            ("Package/Texture/Face.PNG", tiny_png_bytes()),
        ],
    );

    let mut app = build_test_app();
    let handle = load_pmx_handle(&mut app, load_path.to_string_lossy().into_owned());
    wait_for_pmx_asset(&mut app, &handle);

    let assets = app.world().resource::<Assets<Pmx>>();
    let model = assets.get(&handle).expect("should load the PMX asset");
    let images = app.world().resource::<Assets<Image>>();

    assert_eq!(model.texture_paths.len(), 1);
    assert_eq!(model.textures.len(), 1);
    assert_eq!(
        model.texture_paths[0].location(),
        &PmxSourceLocation::zip(zip_path.clone(), "Package/Texture/Face.PNG")
    );

    let image = images
        .get(&model.textures[0])
        .expect("should load the texture subasset");

    assert!(!is_placeholder_texture(image));

    let _ = fs::remove_dir_all(&asset_root);
}

#[test]
fn asset_server_loads_plain_pmx_files_without_regressing_folder_sources() {
    let asset_root = unique_asset_root("bevy_pmx_loader_pmx");
    let model_path = asset_root.join("model.pmx");
    let texture_path = asset_root.join("Texture/Face.PNG");
    let load_path = asset_root
        .strip_prefix("assets")
        .expect("asset path should live under assets")
        .join("model.pmx");
    let pmx_bytes = build_minimal_pmx_bytes(&["Texture/Face.PNG"]);

    write_bytes(&model_path, &pmx_bytes);
    write_bytes(&texture_path, tiny_png_bytes());

    let mut app = build_test_app();
    let handle = load_pmx_handle(&mut app, load_path.to_string_lossy().into_owned());
    wait_for_pmx_asset(&mut app, &handle);

    let assets = app.world().resource::<Assets<Pmx>>();
    let model = assets.get(&handle).expect("should load the PMX asset");
    let images = app.world().resource::<Assets<Image>>();

    assert_eq!(model.texture_paths.len(), 1);
    assert_eq!(
        model.texture_paths[0].location(),
        &PmxSourceLocation::disk(texture_path.clone())
    );

    let image = images
        .get(&model.textures[0])
        .expect("should load the texture subasset");

    assert!(!is_placeholder_texture(image));

    let _ = fs::remove_dir_all(&asset_root);
}

#[test]
fn asset_server_preserves_physics_records_when_disabled() {
    let asset_root = unique_asset_root("bevy_pmx_loader_physics_disabled");
    let model_path = asset_root.join("model.pmx");
    let load_path = asset_root
        .strip_prefix("assets")
        .expect("asset path should live under assets")
        .join("model.pmx");
    let pmx_bytes = build_physics_pmx_bytes();

    write_bytes(&model_path, &pmx_bytes);

    let mut app = build_test_app();
    let handle = load_pmx_handle(&mut app, load_path.to_string_lossy().into_owned());
    wait_for_pmx_asset(&mut app, &handle);

    let assets = app.world().resource::<Assets<Pmx>>();
    let model = assets.get(&handle).expect("should load the PMX asset");
    let rigid_body_assets = app.world().resource::<Assets<PmxRigidBodyRecord>>();
    let joint_assets = app.world().resource::<Assets<PmxJointRecord>>();
    let soft_body_assets = app.world().resource::<Assets<PmxSoftBodyRecord>>();

    assert_eq!(model.rigid_body_records().len(), 1);
    assert_eq!(model.joint_records().len(), 1);
    assert_eq!(model.soft_body_records().len(), 1);
    assert!(model.rigid_body_handles().is_empty());
    assert!(model.joint_handles().is_empty());
    assert!(model.soft_body_handles().is_empty());
    assert!(rigid_body_assets.is_empty());
    assert!(joint_assets.is_empty());
    assert!(soft_body_assets.is_empty());

    let _ = fs::remove_dir_all(&asset_root);
}

#[test]
fn asset_server_materializes_physics_records_when_enabled() {
    let asset_root = unique_asset_root("bevy_pmx_loader_physics");
    let model_path = asset_root.join("model.pmx");
    let load_path = asset_root
        .strip_prefix("assets")
        .expect("asset path should live under assets")
        .join("model.pmx");
    let pmx_bytes = build_physics_pmx_bytes();

    write_bytes(&model_path, &pmx_bytes);

    let mut app = build_physics_test_app();
    let handle = load_pmx_handle(&mut app, load_path.to_string_lossy().into_owned());
    wait_for_pmx_asset(&mut app, &handle);

    let assets = app.world().resource::<Assets<Pmx>>();
    let model = assets.get(&handle).expect("should load the PMX asset");
    let rigid_body_assets = app.world().resource::<Assets<PmxRigidBodyRecord>>();
    let joint_assets = app.world().resource::<Assets<PmxJointRecord>>();
    let soft_body_assets = app.world().resource::<Assets<PmxSoftBodyRecord>>();

    assert_eq!(model.rigid_body_records().len(), 1);
    assert_eq!(model.rigid_body_handles().len(), 1);
    assert_eq!(model.joint_records().len(), 1);
    assert_eq!(model.joint_handles().len(), 1);
    assert_eq!(model.soft_body_records().len(), 1);
    assert_eq!(model.soft_body_handles().len(), 1);
    assert_eq!(
        rigid_body_assets
            .get(&model.rigid_body_handles()[0])
            .expect("should load the rigid body subasset")
            .rigid_body
            .name,
        "rigid"
    );
    assert_eq!(
        joint_assets
            .get(&model.joint_handles()[0])
            .expect("should load the joint subasset")
            .joint
            .name,
        "joint"
    );
    assert_eq!(
        soft_body_assets
            .get(&model.soft_body_handles()[0])
            .expect("should load the soft body subasset")
            .soft_body
            .name,
        "soft"
    );

    let _ = fs::remove_dir_all(&asset_root);
}

#[test]
fn asset_server_uses_placeholders_for_missing_or_bad_zip_textures() {
    let asset_root = unique_asset_root("bevy_pmx_loader_placeholder");
    let zip_path = asset_root.join("character.zip");
    let load_path = asset_root
        .strip_prefix("assets")
        .expect("asset path should live under assets")
        .join("character.zip");
    let pmx_bytes = build_minimal_pmx_bytes(&[
        "Texture/Valid.PNG",
        "Texture/Bad.PNG",
        "Texture/Missing.PNG",
    ]);

    write_zip_at(
        &zip_path,
        vec![
            ("Package/model.pmx", pmx_bytes.as_slice()),
            ("Package/Texture/Valid.PNG", tiny_png_bytes()),
            ("Package/Texture/Bad.PNG", b"not an image"),
        ],
    );

    let mut app = build_test_app();
    let handle = load_pmx_handle(&mut app, load_path.to_string_lossy().into_owned());
    wait_for_pmx_asset(&mut app, &handle);

    let assets = app.world().resource::<Assets<Pmx>>();
    let model = assets.get(&handle).expect("should load the PMX asset");
    let images = app.world().resource::<Assets<Image>>();

    assert_eq!(model.texture_paths.len(), 3);
    let loaded_images: Vec<&Image> = model
        .textures
        .iter()
        .map(|handle| images.get(handle).expect("should load texture subasset"))
        .collect();

    assert!(!is_placeholder_texture(loaded_images[0]));
    assert!(is_placeholder_texture(loaded_images[1]));
    assert!(is_placeholder_texture(loaded_images[2]));

    let _ = fs::remove_dir_all(&asset_root);
}
