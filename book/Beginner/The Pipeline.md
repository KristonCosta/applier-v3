### Writing the shaders
You can follow the tutorial exactly and create the `shaders.wgsl`.

### How do we use the shaders?
There are some differences here on how you set up a pipeline in Bevy vs WGPU. Let's start by preparing our shader. 
#### Preparing the shader
One of the benefits of using Bevy is the automatic asset handling. We want to embed our shader as an asset and be able to have a consistent handle to that asset. To start let's define a basic handle:
```Rust
mod pipeline {
	pub const APPLIER_SHADER_HANDLE: Handle<Shader> =
		Handle::weak_from_u128(154484490495509739857733487233335592041);
}
```

I just generated a random u128 value and generated a `Handle` from it. We can then load the shader as an asset and associate it with this handle:

```Rust
impl Plugin for ApplierPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            APPLIER_SHADER_HANDLE,
            "shaders.wgsl",
            Shader::from_wgsl
        );
	    ...
```

We can use this handle in our pipeline definition. 

#### Defining the pipeline

We can start by defining a resource that will represent our pipeline
```Rust
mod pipeline {
	...
	#[derive(Resource)]
	pub struct ApplierPipeline;
}
```

There are a few options when it comes to constructing our pipeline. Bevy has a number of mechanisms to cache and create specialized pipelines easy, but introducing them all right now may be a bit overwhelming. For now we can use the `PipelineCache` to generate our pipeline and store a reference to it in our resource.

Important note! If you have an HDR display you'll need to change the `format` of your `ColorTargetState` to be `TextureFormat::Bgra8UnormSrgb`. Bevy automatically [configures](https://github.com/bevyengine/bevy/blob/1b3c2b0fed4821d2a8a7554330310ae7f675373d/crates/bevy_render/src/view/window/mod.rs#L441) the target texture based on the capabilities of your display

```Rust

    #[derive(Resource)]
    pub struct ApplierPipeline {
        id: CachedRenderPipelineId, // added a field
    }

    impl FromWorld for ApplierPipeline {
        fn from_world(world: &mut bevy::prelude::World) -> Self {
            let descriptor = RenderPipelineDescriptor {
                vertex: VertexState {
                    shader: APPLIER_SHADER_HANDLE,
                    entry_point: "vs_main".into(),
                    shader_defs: vec![],
                    buffers: vec![],
                },
                fragment: Some(FragmentState {
                    shader: APPLIER_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "fs_main".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                layout: vec![],
                push_constant_ranges: Vec::new(),
                primitive: PrimitiveState {
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                label: Some("applier_pipeline".into()),
            };
			let mut cache = world.resource_mut::<PipelineCache>();
			let id = cache.queue_render_pipeline(descriptor);
			
            Self { id }
        }
    }
}
```

Now let's initialize our resource. The `PipelineCache` is [generated](https://github.com/bevyengine/bevy/blob/1b3c2b0fed4821d2a8a7554330310ae7f675373d/crates/bevy_render/src/lib.rs#L364) in the `finish` step of the `RenderPlugin` so we need to ensure that our new pipeline is only initialized after that completes. We can add it to the `finish` method of our plugin.

```Rust 
fn finish(&self, app: &mut App) {
	if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
		render_app.init_resource::<ApplierPipeline>();
	}
}
```

Now the `SurfaceNode` can fetch the pipeline and use it!

```Rust 

impl Node for SurfaceNode {
        fn run<'w>(
            &self,
            _graph: &mut bevy::render::render_graph::RenderGraphContext,
            render_context: &mut bevy::render::renderer::RenderContext<'w>,
            world: &'w bevy::prelude::World,
        ) -> Result<(), bevy::render::render_graph::NodeRunError> {
            let windows = world.resource::<ExtractedWindows>();
            let mouse_position = world.resource::<MousePosition>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let applier_pipeline = world.resource::<ApplierPipeline>();

            for window in windows.values() {
                if let Some(view) = window.swap_chain_texture_view.as_ref() {
                    let color_attachment = Some(RenderPassColorAttachment {
                        view: view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: (mouse_position.0 as f64 / window.physical_width as f64),
                                g: (mouse_position.1 as f64 / window.physical_height as f64),
                                b: (window.physical_width - mouse_position.0 as f64 / window.physical_width as f64),
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    });
                    let mut render_pass =
                        render_context.begin_tracked_render_pass(RenderPassDescriptor {
                            label: Some("applied_pass"),
                            color_attachments: &[color_attachment],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                    if let Some(pipeline) = pipeline_cache.get_render_pipeline(applier_pipeline.id)
                    {
                        render_pass.set_render_pipeline(pipeline);
                        render_pass.draw(0..3, 0..1);
                    }
                }
            }
            Ok(())
        }

```
Running the program now should show you the brown triangle:

![[brown_triangle.png]]