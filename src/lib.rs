#![feature(async_fn_in_trait)]

use std::mem;
use std::ops;

use bytemuck::NoUninit;
use thiserror::Error;
use tokio::sync::oneshot;

use wgpu::{
    self, include_wgsl, util::DeviceExt, BufferUsages, DeviceType
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to find a GPU! Make sure you have installed required drivers!")]
    GpuNotFound,
}

pub struct Handler {
    device: wgpu::Device,
    module: wgpu::ShaderModule,
    queue: wgpu::Queue,
}

impl Handler {

    /// Initialize the device.
    pub async fn new() -> Result<Self, Error> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or(Error::GpuNotFound)?;

        let adapter_info = adapter.get_info();

        tracing::info!("{adapter_info:?}");

        if matches!(adapter_info.device_type, DeviceType::Cpu) {
            tracing::warn!("Adapter is llvmpipe");
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(env!("CARGO_PKG_NAME")),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let module = device.create_shader_module(include_wgsl!("shader.wgsl"));

        Ok(Self {device, queue, module})
    }
}


mod element {
    /// Prevent others from implementing Element for their own types.
    pub trait Sealed {}
}

/// Valid element types to operate on.
pub trait Element: element::Sealed + NoUninit {}

macro_rules! impl_element {
    ($($ident:ident)+) => {$(
        impl Element for $ident {}
        impl element::Sealed for $ident {}
    )+}
}

impl_element! {
    f32 f64
    i8 i16 i32 i64 isize
    u8 u16 u32 u64 usize
}

pub enum Depth {
    Bridge(u32),
    Result,
}

pub enum Shader {
    Builtin(&'static str), 
    Imported(&'static str),
}

pub struct Context(Depth, Shader);

impl Context {
    
}

// Match case for depth
#[macro_export]
macro_rules! to_depth {
    
    () => {};
    
}

// Match case for shader
#[macro_export]
macro_rules! to_shader {

    (+) => { add };
    (-) => { sub };
    (*) => { mul };
    (/) => { div };
}

// Loop
#[macro_export]
macro_rules! compute_recursive {
    
    ($expr:expr) => { $expr };

    ($tt:tt $op:tt $($expr:expr)*) => {
        Context(to_depth!(), to_shader!($op), $tt, $(compute_recursive!($expr))*)
    };
}

// Parse expression tokens
#[macro_export]
macro_rules! compute {

    ($expr:expr) => {
        compute_recursive!($expr).resolve().await
    };
}

