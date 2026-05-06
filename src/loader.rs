//! Asset loaders for `.lsys` grammar files and material-palette JSON.
//!
//! Enable the `asset-loader` Cargo feature to compile this module. To pick up
//! file edits at runtime, also enable Bevy's `file_watcher` feature on the
//! consuming app (Bevy ships hot-reload behind that flag).
//!
//! # Usage
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_symbios::loader::{LSystemAssetPlugin, LSystemSource, MaterialSettingsSource};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(LSystemAssetPlugin)
//!         .add_systems(Startup, load_assets)
//!         .run();
//! }
//!
//! #[derive(Resource)]
//! struct GrammarHandle(Handle<LSystemSource>);
//!
//! fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.insert_resource(GrammarHandle(asset_server.load("tree.lsys")));
//! }
//! ```

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetApp, AssetLoader, LoadContext};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use symbios::System;

use crate::materials::MaterialSettings;

/// Asset wrapping a parsed [`symbios::System`] grammar.
///
/// Loaded by [`LSystemAssetLoader`] from `.lsys` files.
#[derive(Asset, TypePath)]
pub struct LSystemSource(pub System);

/// Asset wrapping a deserialized material-settings palette.
///
/// Loaded by [`MaterialSettingsAssetLoader`] from `.matpalette.json` files.
/// Apply to a running app by draining into [`crate::materials::MaterialSettingsMap`]
/// and triggering [`crate::materials::MaterialSettingsChanged`].
#[derive(Asset, TypePath)]
pub struct MaterialSettingsSource(pub HashMap<u16, MaterialSettings>);

/// Errors raised while loading an L-System grammar.
#[derive(Debug)]
pub enum LSystemLoaderError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    Parse(symbios::system::SystemError),
}

impl std::fmt::Display for LSystemLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error reading .lsys: {e}"),
            Self::Utf8(e) => write!(f, "Invalid UTF-8 in .lsys: {e}"),
            Self::Parse(e) => write!(f, "L-System parse error: {e:?}"),
        }
    }
}

impl std::error::Error for LSystemLoaderError {}

impl From<std::io::Error> for LSystemLoaderError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::string::FromUtf8Error> for LSystemLoaderError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl From<symbios::system::SystemError> for LSystemLoaderError {
    fn from(e: symbios::system::SystemError) -> Self {
        Self::Parse(e)
    }
}

/// Errors raised while loading a material-palette JSON file.
#[derive(Debug)]
pub enum MaterialSettingsLoaderError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for MaterialSettingsLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error reading material palette: {e}"),
            Self::Json(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}

impl std::error::Error for MaterialSettingsLoaderError {}

impl From<std::io::Error> for MaterialSettingsLoaderError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for MaterialSettingsLoaderError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Loader for `.lsys` files.
#[derive(Default, TypePath)]
pub struct LSystemAssetLoader;

impl AssetLoader for LSystemAssetLoader {
    type Asset = LSystemSource;
    type Settings = ();
    type Error = LSystemLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<LSystemSource, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let source = String::from_utf8(bytes)?;
        let system = parse_lsys_source(&source)?;
        Ok(LSystemSource(system))
    }

    fn extensions(&self) -> &[&str] {
        &["lsys"]
    }
}

/// Loader for material-palette JSON files (extension `.matpalette.json`).
#[derive(Default, TypePath)]
pub struct MaterialSettingsAssetLoader;

impl AssetLoader for MaterialSettingsAssetLoader {
    type Asset = MaterialSettingsSource;
    type Settings = ();
    type Error = MaterialSettingsLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<MaterialSettingsSource, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let map = parse_material_settings(&bytes)?;
        Ok(MaterialSettingsSource(map))
    }

    fn extensions(&self) -> &[&str] {
        &["matpalette.json"]
    }
}

/// Pure parser for `.lsys` source strings — exposed for tests and CLI tools.
pub fn parse_lsys_source(src: &str) -> Result<System, symbios::system::SystemError> {
    System::from_source(src)
}

/// Pure parser for material-palette JSON bytes — exposed for tests and CLI tools.
pub fn parse_material_settings(
    bytes: &[u8],
) -> Result<HashMap<u16, MaterialSettings>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// Plugin that registers the [`LSystemSource`] / [`MaterialSettingsSource`]
/// assets and their loaders.
pub struct LSystemAssetPlugin;

impl Plugin for LSystemAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LSystemSource>()
            .init_asset::<MaterialSettingsSource>()
            .init_asset_loader::<LSystemAssetLoader>()
            .init_asset_loader::<MaterialSettingsAssetLoader>();
    }
}
