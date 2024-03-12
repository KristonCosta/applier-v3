use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_asset::RenderAssets,
        render_graph::{RenderGraph, RenderGraphApp},
        render_resource::{AsBindGroup, BufferVec},
        renderer::{RenderDevice, RenderQueue},
        texture::FallbackImage,
        Extract, Render, RenderApp, RenderSet,
    },
};
use wgpu::BufferUsages;

use crate::plugin::pipeline::{ApplierPipeline, APPLIER_SHADER_HANDLE};

use self::{
    material::{ApplierMaterial, PreparedApplierMaterial},
    node::SurfaceNode,
};

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
    use std::mem;

    use bevy::render::render_resource::VertexBufferLayout;
    use bytemuck::{Pod, Zeroable};
    use wgpu::{BufferAddress, VertexAttribute, VertexFormat, VertexStepMode};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct Vertex {
        position: [f32; 3],
        tex_coords: [f32; 2], // NEW!
    }

    pub const VERTICES: &[Vertex] = &[
        // Changed
        Vertex {
            position: [-0.0868241, 0.49240386, 0.0],
            tex_coords: [0.4131759, 0.00759614],
        }, // A
        Vertex {
            position: [-0.49513406, 0.06958647, 0.0],
            tex_coords: [0.0048659444, 0.43041354],
        }, // B
        Vertex {
            position: [-0.21918549, -0.44939706, 0.0],
            tex_coords: [0.28081453, 0.949397],
        }, // C
        Vertex {
            position: [0.35966998, -0.3473291, 0.0],
            tex_coords: [0.85967, 0.84732914],
        }, // D
        Vertex {
            position: [0.44147372, 0.2347359, 0.0],
            tex_coords: [0.9414737, 0.2652641],
        }, // E
    ];

    pub const INDICES: &[u32] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];

    impl Vertex {
        pub fn desc() -> VertexBufferLayout {
            VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2, // NEW!
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
        graph::ApplierSubgraph, material::PreparedApplierMaterial, pipeline::ApplierPipeline,
        IndexBuffer, MousePosition, VertexBuffer,
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
            let bind_group = world.resource::<PreparedApplierMaterial>();

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
                        render_pass.set_bind_group(0, &bind_group.bind_group, &[]);
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

mod material {
    use bevy::{
        asset::{AssetServer, Handle},
        ecs::{system::Resource, world::FromWorld},
        render::{
            render_resource::{AsBindGroup, BindGroup, OwnedBindingResource},
            renderer::RenderDevice,
            texture::{FallbackImage, Image},
        },
    };

    use super::pipeline::ApplierPipeline;

    #[derive(AsBindGroup, Resource)]
    pub struct ApplierMaterial {
        #[texture(0)]
        #[sampler(1)]
        pub image: Handle<Image>,
    }

    impl FromWorld for ApplierMaterial {
        fn from_world(world: &mut bevy::prelude::World) -> Self {
            let asset_server = world.resource::<AssetServer>();
            let handle = asset_server.load("tree.png");
            Self { image: handle }
        }
    }

    #[derive(Resource)]
    pub struct PreparedApplierMaterial {
        pub bindings: Vec<(u32, OwnedBindingResource)>,
        pub bind_group: BindGroup,
    }
}

mod pipeline {
    use bevy::{
        asset::Handle,
        ecs::{system::Resource, world::FromWorld},
        render::{
            render_resource::{
                AsBindGroup, BindGroupLayout, CachedRenderPipelineId, FragmentState, PipelineCache,
                RenderPipelineDescriptor, Shader, VertexState,
            },
            renderer::RenderDevice,
        },
    };
    use wgpu::{
        BlendState, ColorTargetState, ColorWrites, Face, FrontFace, MultisampleState, PolygonMode,
        PrimitiveState, PrimitiveTopology, TextureFormat,
    };

    use super::{material::ApplierMaterial, mesh::Vertex};

    pub const APPLIER_SHADER_HANDLE: Handle<Shader> =
        Handle::weak_from_u128(154484490495509739857733487233335592041);

    #[derive(Resource)]
    pub struct ApplierPipeline {
        pub id: CachedRenderPipelineId,
        pub material_layout: BindGroupLayout,
    }

    impl FromWorld for ApplierPipeline {
        fn from_world(world: &mut bevy::prelude::World) -> Self {
            let render_device = world.resource::<RenderDevice>();
            let material_layout = ApplierMaterial::bind_group_layout(render_device);
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
                layout: vec![material_layout.clone()],
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

            Self {
                id,
                material_layout,
            }
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
            .init_resource::<ApplierMaterial>()
            .add_systems(Update, (cursor_events,));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(MousePosition(0.0, 0.0))
                .init_resource::<VertexBuffer>()
                .init_resource::<IndexBuffer>()
                .add_systems(ExtractSchedule, (extract_mouse_position, extract_material))
                .add_systems(
                    Render,
                    (
                        prepare_buffers.in_set(RenderSet::PrepareResources),
                        prepare_bind_groups.in_set(RenderSet::PrepareResources),
                    ),
                );

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

fn extract_material(
    mut commands: Commands,
    extracted_material: Option<Res<ApplierMaterial>>,
    main_material: Extract<Res<ApplierMaterial>>,
) {
    if extracted_material.is_none() {
        commands.insert_resource(ApplierMaterial {
            image: main_material.image.clone(),
        })
    }
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

fn prepare_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    material: Res<ApplierMaterial>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    prepared_material: Option<Res<PreparedApplierMaterial>>,
    pipeline: Res<ApplierPipeline>,
) {
    if prepared_material.is_none() {
        let prepared = material
            .as_bind_group(
                &pipeline.material_layout,
                &render_device,
                &images,
                &fallback_image,
            )
            .expect("failed to prepare bind group");

        commands.insert_resource(PreparedApplierMaterial {
            bindings: prepared.bindings,
            bind_group: prepared.bind_group,
        });
    }
}
