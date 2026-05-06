//! Demonstrates the `asset-loader` feature: load `.lsys` and
//! `.matpalette.json` files, and observe live edits via Bevy's file watcher.
//!
//! Run with:
//! ```sh
//! cargo run --example hot_reload --features asset-loader,bevy/file_watcher
//! ```
//!
//! Then edit `assets/example_tree.lsys` or `assets/example_palette.matpalette.json`
//! while the app is running — the console will report each reload.

use bevy::prelude::*;
use bevy_symbios::loader::{LSystemAssetPlugin, LSystemSource, MaterialSettingsSource};

#[derive(Resource)]
struct LoadedHandles {
    _grammar: Handle<LSystemSource>,
    _palette: Handle<MaterialSettingsSource>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LSystemAssetPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, report_changes)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LoadedHandles {
        _grammar: asset_server.load("example_tree.lsys"),
        _palette: asset_server.load("example_palette.matpalette.json"),
    });
    info!("Loaded example_tree.lsys and example_palette.matpalette.json — try editing them!");
}

fn report_changes(
    mut grammar_events: MessageReader<AssetEvent<LSystemSource>>,
    mut palette_events: MessageReader<AssetEvent<MaterialSettingsSource>>,
    palettes: Res<Assets<MaterialSettingsSource>>,
) {
    for ev in grammar_events.read() {
        if let AssetEvent::Added { id } | AssetEvent::Modified { id } = ev {
            info!("L-System grammar (re)loaded: id={id:?}");
        }
    }
    for ev in palette_events.read() {
        if let AssetEvent::Added { id } | AssetEvent::Modified { id } = ev {
            if let Some(palette) = palettes.get(*id) {
                info!("Material palette (re)loaded: {} entries", palette.0.len());
            }
        }
    }
}
