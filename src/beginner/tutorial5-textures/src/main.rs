mod plugin;

use bevy::prelude::*;
use plugin::{material::ApplierMaterial, ApplierPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        ApplierPlugin,
    ));
    app.add_systems(Startup, setup).run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ApplierMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let handle = asset_server.load("tree.png");
    let material = materials.add(ApplierMaterial { image: handle });

    commands.spawn((material,));
}
