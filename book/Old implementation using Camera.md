```Rust
pub mod camera {
    use bevy::{
        ecs::bundle::Bundle,
        render::{
            camera::{Camera, CameraMainTextureUsages, CameraRenderGraph, Exposure, Projection},
            primitives::Frustum,
            view::{ColorGrading, VisibleEntities},
        },
        transform::components::{GlobalTransform, Transform},
    };

    use super::graph;

    #[derive(Bundle)]
    pub struct CameraApplierBundle {
        pub camera: Camera,
        pub camera_render_graph: CameraRenderGraph,
        pub global_transform: GlobalTransform,
        pub visible_entities: VisibleEntities,
        pub frustum: Frustum,
        pub projection: Projection,
    }

    impl Default for CameraApplierBundle {
        fn default() -> Self {
            Self {
                camera: Default::default(),
                camera_render_graph: CameraRenderGraph::new(graph::ApplierSubgraph),
                projection: Default::default(),
                visible_entities: Default::default(),
                frustum: Default::default(),
                global_transform: Default::default(),
            }
        }
    }
}
```
### Registering our subgraph
Cameras aren't introduced until [tutorial 6](https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#a-perspective-camera) of the WGPU tutorial so it may seem odd to introduce Bevy's camera now but it greatly simplifies a few things for us, including scheduling our new subgraph.

A camera is tied to a particular render graph. When a camera entity is processed a top level render node is automatically created which executes the render graph associated with the camera. 

In order to do this all we need to do is define a new type of Camera bundle
```Rust
pub struct CameraApplierBundle {
	pub camera: Camera,
	pub camera_render_graph: CameraRenderGraph,
	// You might be asking "why do we need all of these?"
	// The quick answer is they're mandatory in order to actually 
	// get the camera automatically extracted out from our main
	// app into the render app. 
	// 
	// We haven't gone over the concept of `extract` yet, but
	// you can see the query used to extract camera information 
	// over here https://github.com/bevyengine/bevy/blob/f0a98645d0dcc3dc76ad335755adb074f6fbe5db/crates/bevy_render/src/camera/camera.rs#L804-L816
	pub global_transform: GlobalTransform,
	pub visible_entities: VisibleEntities,
	pub frustum: Frustum,
	pub projection: Projection,
}

  

impl Default for CameraApplierBundle {
	fn default() -> Self {
		Self {
			camera: Default::default(),
			camera_render_graph: CameraRenderGraph::new(graph::ApplierSubgraph),
			projection: Default::default(),
			visible_entities: Default::default(),
			frustum: Default::default(),
			global_transform: Default::default(),
		}
	}
}

```
and then just spawn a camera in our main function
```
fn main() {
	App::new()
		.add_plugins((
			ApplierPlugin,
			DefaultPlugins.set(ImagePlugin::default_nearest()),
		))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands.spawn(CameraApplierBundle::default());
}
```

Now our program fails when we run it
```
thread '<unnamed>' panicked at src/beginner/tutorial2-surface/src/plugin.rs:34:13:
not yet implemented
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Encountered a panic in exclusive system `bevy_render::renderer::render_system`!
```

