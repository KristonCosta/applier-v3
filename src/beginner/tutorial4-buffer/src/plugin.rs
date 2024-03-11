use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderGraphApp},
        render_resource::BufferVec,
        renderer::{RenderDevice, RenderQueue},
        Extract, Render, RenderApp, RenderSet,
    },
};
use wgpu::BufferUsages;

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

mod mesh {
    use bevy::render::render_resource::VertexBufferLayout;
    use bytemuck::{Pod, Zeroable};
    use wgpu::{BufferAddress, VertexAttribute, VertexFormat, VertexStepMode};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Pod, Zeroable)]
    pub struct Vertex {
        pub position: [f32; 3],
        pub color: [f32; 3],
    }

    pub const VERTICES: &[Vertex] = &[
        Vertex {
            position: [-0.0868241, 0.49240386, 0.0],
            color: [0.5, 0.0, 0.5],
        }, // A
        Vertex {
            position: [-0.49513406, 0.06958647, 0.0],
            color: [0.5, 0.0, 0.5],
        }, // B
        Vertex {
            position: [-0.21918549, -0.44939706, 0.0],
            color: [0.5, 0.0, 0.5],
        }, // C
        Vertex {
            position: [0.35966998, -0.3473291, 0.0],
            color: [0.5, 0.0, 0.5],
        }, // D
        Vertex {
            position: [0.44147372, 0.2347359, 0.0],
            color: [0.5, 0.0, 0.5],
        }, // E
    ];

    pub const INDICES: &[u32] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];

    impl Vertex {
        pub fn desc() -> VertexBufferLayout {
            VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![
                    VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: VertexFormat::Float32x3,
                    },
                    VertexAttribute {
                        offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                        shader_location: 1,
                        format: VertexFormat::Float32x3,
                    },
                ],
            }
        }
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

    use super::{
        graph::ApplierSubgraph, pipeline::ApplierPipeline, IndexBuffer, MousePosition, VertexBuffer,
    };

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
            let vertex_buffer = world.resource::<VertexBuffer>();
            let index_buffer = world.resource::<IndexBuffer>();

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
                        render_pass.set_vertex_buffer(
                            0,
                            vertex_buffer
                                .0
                                .buffer()
                                .expect("buffer was not set")
                                .slice(..),
                        );
                        render_pass.set_index_buffer(
                            index_buffer
                                .0
                                .buffer()
                                .expect("buffer was not set")
                                .slice(..),
                            0,
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..index_buffer.0.len() as u32, 0, 0..1)
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
        render::render_resource::{
            CachedRenderPipelineId, FragmentState, PipelineCache, RenderPipelineDescriptor, Shader,
            VertexState,
        },
    };
    use wgpu::{
        BlendState, ColorTargetState, ColorWrites, Face, FrontFace, MultisampleState, PolygonMode,
        PrimitiveState, PrimitiveTopology, TextureFormat,
    };

    use super::mesh::Vertex;

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
                    buffers: vec![Vertex::desc()],
                },
                fragment: Some(FragmentState {
                    shader: APPLIER_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "fs_main".into(),
                    targets: vec![Some(ColorTargetState {
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
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(MousePosition(0.0, 0.0))
                .init_resource::<VertexBuffer>()
                .init_resource::<IndexBuffer>()
                .add_systems(ExtractSchedule, extract_mouse_position)
                .add_systems(Render, prepare_buffers.in_set(RenderSet::PrepareResources));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
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
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<ApplierPipeline>();
        }
    }
}

#[derive(Resource)]
pub struct VertexBuffer(BufferVec<mesh::Vertex>);

impl FromWorld for VertexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = BufferVec::new(BufferUsages::VERTEX);
        buff.extend(mesh::VERTICES.to_vec());
        Self(buff)
    }
}

#[derive(Resource)]
pub struct IndexBuffer(BufferVec<u32>);

impl FromWorld for IndexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = BufferVec::new(BufferUsages::INDEX);
        buff.extend(mesh::INDICES.to_vec());
        Self(buff)
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

fn prepare_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut vertex_buffer: ResMut<VertexBuffer>,
    mut index_buffer: ResMut<IndexBuffer>,
) {
    vertex_buffer.0.write_buffer(&render_device, &render_queue);
    index_buffer.0.write_buffer(&render_device, &render_queue);
}
