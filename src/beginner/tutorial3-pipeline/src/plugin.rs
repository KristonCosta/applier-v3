use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderGraphApp},
        Extract, RenderApp,
    },
};

use crate::plugin::pipeline::{ApplierPipeline, APPLIER_SHADER_HANDLE};

use self::node::SurfaceNode;

pub struct ApplierPlugin;

mod graph {
    use bevy::render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct ApplierSubgraph;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum ApplierNode {
        ExecuteNode,
        SurfaceNode,
    }
}

mod node {
    use bevy::{
        ecs::world::FromWorld,
        render::{
            render_graph::Node,
            render_resource::{
                LoadOp, Operations, PipelineCache, RenderPassColorAttachment, StoreOp,
            },
            view::ExtractedWindows,
        },
    };
    use wgpu::{Color, RenderPassDescriptor};

    use super::{graph::ApplierSubgraph, pipeline::ApplierPipeline, MousePosition};

    pub struct SurfaceNode;

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
                                b: ((window.physical_width as f64 - mouse_position.0 as f64)
                                    / window.physical_width as f64),
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
    }

    impl FromWorld for SurfaceNode {
        fn from_world(_world: &mut bevy::prelude::World) -> Self {
            SurfaceNode
        }
    }

    pub struct ExecuteNode;

    impl Node for ExecuteNode {
        fn run<'w>(
            &self,
            graph: &mut bevy::render::render_graph::RenderGraphContext,
            _render_context: &mut bevy::render::renderer::RenderContext<'w>,
            _world: &'w bevy::prelude::World,
        ) -> Result<(), bevy::render::render_graph::NodeRunError> {
            graph.run_sub_graph(ApplierSubgraph, vec![], None)?;
            Ok(())
        }
    }
}

mod pipeline {
    use bevy::{
        asset::Handle,
        ecs::{system::Resource, world::FromWorld},
        render::{
            render_resource::{
                CachedRenderPipelineId, FragmentState, PipelineCache, RenderPipelineDescriptor,
                Shader, VertexState,
            },
            // texture::BevyDefault,
        },
    };
    use wgpu::{
        BlendState, ColorTargetState, ColorWrites, Face, FrontFace, MultisampleState, PolygonMode,
        PrimitiveState, PrimitiveTopology, TextureFormat,
    };

    pub const APPLIER_SHADER_HANDLE: Handle<Shader> =
        Handle::weak_from_u128(154484490495509739857733487233335592041);

    #[derive(Resource)]
    pub struct ApplierPipeline {
        pub id: CachedRenderPipelineId,
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
                        // format: TextureFormat::bevy_default(),
                        format: TextureFormat::Bgra8UnormSrgb,
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
            let cache = world.resource_mut::<PipelineCache>();
            let id = cache.queue_render_pipeline(descriptor);

            Self { id }
        }
    }
}

impl Plugin for ApplierPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            APPLIER_SHADER_HANDLE,
            "shaders.wgsl",
            Shader::from_wgsl
        );
        app.insert_resource(MousePosition(0.0, 0.0))
            .add_systems(Update, (cursor_events,));
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(MousePosition(0.0, 0.0))
                .add_systems(ExtractSchedule, extract_mouse_position);

            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(graph::ApplierNode::ExecuteNode, node::ExecuteNode);

            render_graph
                .remove_node(CameraDriverLabel)
                .expect("failed to remove camera driver");

            render_app
                .add_render_sub_graph(graph::ApplierSubgraph)
                .add_render_graph_node::<SurfaceNode>(
                    graph::ApplierSubgraph,
                    graph::ApplierNode::SurfaceNode,
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<ApplierPipeline>();
        }
    }
}

fn extract_mouse_position(
    mut mouse_position: ResMut<MousePosition>,
    main_mouse_position: Extract<Res<MousePosition>>,
) {
    mouse_position.0 = main_mouse_position.0;
    mouse_position.1 = main_mouse_position.1;
}

#[derive(Resource, Debug)]
pub struct MousePosition(f32, f32);

fn cursor_events(
    mut events: EventReader<CursorMoved>,
    mut current_position: ResMut<MousePosition>,
) {
    for event in events.read() {
        current_position.0 = event.position.x;
        current_position.1 = event.position.y;
    }
}
