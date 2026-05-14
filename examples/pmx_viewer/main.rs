//! Load a PMX file from the command line and open a Bevy window to inspect it.
//!
//! This example intentionally uses the crate's public API instead of any private internals:
//! `parse_pmx` parses the file, `import_pmx` builds the Bevy-friendly model, and
//! `PmxMeshGeometry::to_mesh()` turns the imported geometry into a renderable mesh.

mod gizmos;
mod orbit;
mod scene;

use std::error::Error;

use bevy::{
    pbr::wireframe::WireframePlugin,
    prelude::*,
    window::{Window, WindowPlugin},
};
use bevy_pmx::prelude::*;
use clap::Parser;

use crate::scene::{SceneRequest, bootstrap_scene};

#[derive(Debug, Parser)]
#[command(
    name = "pmx_viewer",
    version,
    about = "Load a PMX file and open a viewer window"
)]
struct Cli {
    /// Path to the PMX file to open.
    #[arg(value_name = "PMX")]
    path: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    App::new()
        .insert_resource(SceneRequest { path: cli.path })
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        })
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy_pmx".to_owned(),
                    ..default()
                }),
                ..default()
            }),
            PmxPlugin::default(),
            WireframePlugin::default(),
        ))
        .add_plugins(orbit::OrbitCameraPlugin)
        .add_plugins(gizmos::ViewerGizmoPlugin)
        .add_systems(Startup, bootstrap_scene)
        .run();

    Ok(())
}
