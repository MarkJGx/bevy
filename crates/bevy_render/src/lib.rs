pub mod batch;
pub mod camera;
pub mod color;
pub mod draw;
pub mod dispatch;
pub mod mesh;
pub mod pass;
pub mod pipeline;
pub mod render_graph;
pub mod renderer;
pub mod shader;
pub mod texture;

mod entity;
pub use once_cell;

pub mod prelude {
    pub use crate::{
        base::Msaa,
        color::Color,
        draw::Draw,
        entity::*,
        mesh::{shape, Mesh},
        pipeline::RenderPipelines,
        shader::Shader,
        texture::Texture,
    };
}

use crate::prelude::*;
use base::{MainPass, Msaa};
use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::{IntoQuerySystem, IntoThreadLocalSystem};
use bevy_type_registry::RegisterType;
use camera::{
    ActiveCameras, Camera, OrthographicProjection, PerspectiveProjection, VisibleEntities,
};
use pipeline::{
    ComputePipelineCompiler, DynamicBinding, PipelineCompiler, PipelineDescriptor, PipelineSpecialization,
    PrimitiveTopology, ShaderSpecialization, VertexBufferDescriptors, ComputePipelineSpecialization, ComputePipelineDescriptor,
};
use render_graph::{
    base::{self, BaseRenderGraphBuilder, BaseRenderGraphConfig},
    RenderGraph,
};
use renderer::{AssetRenderResourceBindings, RenderResourceBindings};
use std::ops::Range;
use texture::{HdrTextureLoader, ImageTextureLoader, TextureResourceSystemState};

/// The names of "render" App stages
pub mod stage {
    /// Stage where render resources are set up
    pub static RENDER_RESOURCE: &str = "render_resource";
    /// Stage where Render Graph systems are run. In general you shouldn't add systems to this stage manually.
    pub static RENDER_GRAPH_SYSTEMS: &str = "render_graph_systems";
    /// Compute stage where compute systems are executed.
    pub static COMPUTE: &str = "compute";
    // Stage where draw systems are executed. This is generally where Draw components are setup
    pub static DRAW: &str = "draw";
    pub static RENDER: &str = "render";
    pub static POST_RENDER: &str = "post_render";
}

/// Adds core render types and systems to an App
pub struct RenderPlugin {
    /// configures the "base render graph". If this is not `None`, the "base render graph" will be added  
    pub base_render_graph_config: Option<BaseRenderGraphConfig>,
}

impl Default for RenderPlugin {
    fn default() -> Self {
        RenderPlugin {
            base_render_graph_config: Some(BaseRenderGraphConfig::default()),
        }
    }
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(bevy_asset::stage::ASSET_EVENTS, stage::RENDER_RESOURCE)
            .add_stage_after(stage::RENDER_RESOURCE, stage::RENDER_GRAPH_SYSTEMS)
            .add_stage_after(stage::RENDER_GRAPH_SYSTEMS, stage::COMPUTE)
            .add_stage_after(stage::RENDER_GRAPH_SYSTEMS, stage::DRAW)
            .add_stage_after(stage::DRAW, stage::RENDER)
            .add_stage_after(stage::RENDER, stage::POST_RENDER)
            .add_asset::<Mesh>()
            .add_asset::<Texture>()
            .add_asset::<Shader>()
            .add_asset::<PipelineDescriptor>()
            .add_asset::<ComputePipelineDescriptor>()
            .add_asset_loader::<Texture, HdrTextureLoader>()
            .add_asset_loader::<Texture, ImageTextureLoader>()
            .register_component::<Camera>()
            .register_component::<Draw>()
            .register_component::<RenderPipelines>()
            .register_component::<OrthographicProjection>()
            .register_component::<PerspectiveProjection>()
            .register_component::<MainPass>()
            .register_component::<VisibleEntities>()
            .register_property::<Color>()
            .register_property::<Range<f32>>()
            .register_property::<ShaderSpecialization>()
            .register_property::<DynamicBinding>()
            .register_property::<PrimitiveTopology>()
            .register_properties::<PipelineSpecialization>()
            .register_properties::<ComputePipelineSpecialization>()
            .init_resource::<RenderGraph>()
            .init_resource::<PipelineCompiler>()
            .init_resource::<ComputePipelineCompiler>()
            .init_resource::<RenderResourceBindings>()
            .init_resource::<VertexBufferDescriptors>()
            .init_resource::<TextureResourceSystemState>()
            .init_resource::<AssetRenderResourceBindings>()
            .init_resource::<ActiveCameras>()
            .add_system_to_stage(
                bevy_app::stage::PRE_UPDATE,
                draw::clear_draw_system.system(),
            )
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::active_cameras_system.system(),
            )
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<OrthographicProjection>.system(),
            )
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<PerspectiveProjection>.system(),
            )
            // registration order matters here. this must come after all camera_system::<T> systems
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::visible_entities_system.system(),
            )
            // TODO: turn these "resource systems" into graph nodes and remove the RENDER_RESOURCE stage
            .add_system_to_stage(
                stage::RENDER_RESOURCE,
                mesh::mesh_resource_provider_system.system(),
            )
            .add_system_to_stage(
                stage::RENDER_RESOURCE,
                Texture::texture_resource_system.system(),
            )
            .add_system_to_stage(
                stage::RENDER_GRAPH_SYSTEMS,
                render_graph::render_graph_schedule_executor_system.thread_local_system(),
            )
            .add_system_to_stage(stage::COMPUTE, pipeline::dispatch_compute_pipelines_system.system())
            .add_system_to_stage(stage::DRAW, pipeline::draw_render_pipelines_system.system())
            .add_system_to_stage(
                stage::POST_RENDER,
                shader::clear_shader_defs_system.system(),
            );

        if app.resources().get::<Msaa>().is_none() {
            app.init_resource::<Msaa>();
        }

        if let Some(ref config) = self.base_render_graph_config {
            let resources = app.resources();
            let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
            let msaa = resources.get::<Msaa>().unwrap();
            render_graph.add_base_graph(config, &msaa);
            let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
            if config.add_3d_camera {
                active_cameras.add(base::camera::CAMERA3D);
            }

            if config.add_2d_camera {
                active_cameras.add(base::camera::CAMERA2D);
            }
        }
    }
}
