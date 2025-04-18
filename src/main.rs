#[allow(dead_code)] mod track; use track::*;
#[allow(dead_code)] mod load; use load::*;
#[allow(dead_code)] mod player; use player::*;
#[allow(dead_code)] mod filter; use filter::*;

use std::sync::Arc;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use symphonia::core::{audio::{AudioBufferRef, Signal}, codecs::DecoderOptions, conv::IntoSample, formats::FormatOptions, io::{MediaSourceStream, MediaSourceStreamOptions}, meta::MetadataOptions, probe::Hint};
use rubato::Resampler;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, KeyEvent, WindowEvent}, event_loop::ControlFlow, keyboard::{KeyCode, PhysicalKey}, window::Window};



const RESAMPLER_BLOCK_SIZE: usize = 1024;


pub struct WindowState<'a> {
	pub surface: Surface<'a>,
	pub device: Device,
	pub queue: Queue,
	pub config: SurfaceConfiguration,
	pub size: PhysicalSize<u32>,
	pub render_pipeline: wgpu::RenderPipeline,
}

impl<'a> WindowState<'a> {
	pub async fn new(window: Arc<Window>) -> Self {
		
		let size = window.inner_size();
		
		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::VULKAN,
			..Default::default()
		});
		
		let surface = instance.create_surface(window).unwrap();
		
		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::LowPower,
			compatible_surface: Some(&surface),
			force_fallback_adapter: false,
		}).await.unwrap();
		
		let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
			required_features: wgpu::Features::empty(),
			required_limits: wgpu::Limits::default(),
			label: None,
			memory_hints: Default::default(),
		}, None).await.unwrap();
		
		let surface_caps = surface.get_capabilities(&adapter);
		let surface_format = surface_caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_caps.formats[0]);
		
		
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};
		
		
		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("shader"),
			source: wgpu::ShaderSource::Wgsl(include_str!("shaders/test.wgsl").into()),
		});
		
		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("pipeline layout lol"),
			bind_group_layouts: &[],
			push_constant_ranges: &[],
		});
		
		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("render pipeline lol"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
			cache: None,
		});
		
		
		Self {
			surface,
			device,
			queue,
			config,
			size,
			render_pipeline,
		}
	}
}

impl<'a> WindowState<'a> {
	pub fn resize(&mut self, size: PhysicalSize<u32>) {
		if size.width > 0 && size.height > 0 {
			self.config.width = size.width;
			self.config.height = size.height;
			self.surface.configure(&self.device, &self.config);
			self.size = size;
		}
	}
}



pub struct App<'a> {
	pub window_state: Option<(Arc<Window>, WindowState<'a>)>,
	pub audio_player: AudioPlayer,
	pub mouse_pos: PhysicalPosition<f64>,
}


impl<'a> App<'a> {
	pub fn new() -> Self {
		let device = cpal::default_host().default_output_device().expect("No default device found");
		let audio_player = AudioPlayer::new(device);
		
		Self {
			window_state: None,
			audio_player,
			mouse_pos: PhysicalPosition { x: 0.0, y: 0.0 }
		}
	}
}


impl<'a> ApplicationHandler for App<'a> {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		let attributes = Window::default_attributes()
			.with_title("sfx daw")
			.with_inner_size(PhysicalSize::new(1440, 810));
		
		let window = Arc::new(event_loop.create_window(attributes).unwrap());
		
		self.window_state = Some((
			Arc::clone(&window),
			futures_lite::future::block_on(WindowState::new(Arc::clone(&window))),
		));
		
		
		
		
	}
	
	fn window_event(
			&mut self,
			event_loop: &winit::event_loop::ActiveEventLoop,
			_window_id: winit::window::WindowId,
			event: winit::event::WindowEvent,
		) {
		
		let Some((ref window, ref mut ws)) = self.window_state.as_mut() else { return };
		
		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			
			WindowEvent::Resized(size) => {
				ws.resize(size);
			}
			
			WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(key_code), state, repeat: _, .. }, .. } => match key_code {
				KeyCode::Escape => if state.is_pressed() { event_loop.exit() }
				KeyCode::Space => if state.is_pressed() { println!("space") }
				_ => ()
			}
			
			WindowEvent::CursorMoved { position, device_id: _ } => {
				self.mouse_pos = position;
			}
			
			WindowEvent::RedrawRequested => {
				
				
				
				let output = match ws.surface.get_current_texture() {
					Ok(output) => output,
					Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
						ws.resize(ws.size);
						return
					}
					_ => todo!()
				};
				
				let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
				
				let mut encoder = ws.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
					label: Some("Render Encoder"),
				});
				
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					label: Some("Render Pass"),
					color_attachments: &[Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: self.mouse_pos.x / ws.size.width as f64, b: 0.1, a: 1.0 }),
							store: wgpu::StoreOp::Store,
						},
					})],
					depth_stencil_attachment: None,
					occlusion_query_set: None,
					timestamp_writes: None,
				});
				
				render_pass.set_pipeline(&ws.render_pipeline);
				render_pass.draw(0..3, 0..1);
				
				drop(render_pass);
				
				ws.queue.submit(std::iter::once(encoder.finish()));
				output.present();
				
				window.request_redraw();
			}
			
			_ => ()
		}
		
	}
}






#[allow(deprecated)]
fn main() {
	
	let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
	event_loop.set_control_flow(ControlFlow::Poll);
	let mut app = App::new();
	
	
	
	// let dir = std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pink Floyd/The Wall [Disc 1]");
	// let mut files = vec![];
	// for entry in std::fs::read_dir(dir).unwrap() {
	// 	let entry = entry.unwrap();
	// 	if entry.metadata().unwrap().is_file() {
	// 		files.push(entry.path());
	// 	}
	// }
	// let audio = load_audio(&files).unwrap();
	
	
	// let audio = load_audio(&[
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/01 Life In Technicolor.flac"),
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/02 Cemeteries of London.flac"),
	// ]).unwrap();
	
	// let audio = load_audio(&[
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pierce The Veil/Collide With The Sky/01 May These Noises Startle You In Your Sleep Tonight.flac"),
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pierce The Veil/Collide With The Sky/02 Hell Above.flac"),
	// ]).unwrap();
	
	// let filtered_audio = test_filter(&audio);
	
	
	
	// let tracks = audio_player.add_tracks(audio.into_iter());
	
	// audio_player.play_track(tracks[0]);
	// for i in 1..tracks.len() { audio_player.queue_track(tracks[i]); }
	
	// audio_player.seek(70.0);
	
	
	event_loop.run_app(&mut app).unwrap();
}
