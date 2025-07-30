mod plugin;

use bevy::prelude::*;
use cgmath::{InnerSpace, Rotation3, Zero};
use plugin::ApplierPlugin;

use crate::plugin::mesh::{ApplierMesh, ApplierMesh3d};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        ApplierPlugin,
    ))
    .add_systems(Startup, (setup,));
    app.run();
}

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);
const SPACE_BETWEEN: f32 = 3.0;

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>
) {
    let handle: Handle<ApplierMesh> = server.load("cube.obj");
    // let transform = ;
    
    let transforms: Vec<Transform> = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position =
                        SPACE_BETWEEN * cgmath::Vector3::new(x as f32, 0.0, z as f32) - INSTANCE_DISPLACEMENT;
                    let mut transform = Transform::from_xyz(position.x, position.y, position.z);
                    
                    let rotation = if position.is_zero() {
                        Quat::from_axis_angle(Vec3::Y, cgmath::Deg(0.0).0 as f32)
                    } else {
                        Quat::from_axis_angle(transform.translation.normalize(), cgmath::Deg(45.0).0 as f32)
                    };
                    transform.rotation = rotation;
                    transform
                })
            })
            .collect();
    for transform in transforms {
        commands.spawn((transform, ApplierMesh3d(handle.clone())));
    }
}
