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

pub enum Depth { 
    Entry,
    Bridge(u32),
    Result,
}

pub enum Identity {
    Unique, 
    Shared(u32),
}

pub enum Shader {
    Builtin(&'static str),
    Path(&'static str),
}

pub trait Resolvable {
    type Output;
}

pub struct Component<T: Element, const N: usize> {
    local: [T; N], 
    identity: Identity,
}

impl<T: Element, const N: usize> Resolvable for Component<T, N> {
    type Output = Component<T, N>;
}

pub struct Context<F: Element, S: Element, const L: usize, const R: usize> {
    shader: Shader,
    lhs: Component<F, L>, 
    rhs: Component<S, R>,
    depth: Depth,
}

impl<F: Element, S: Element, const L: usize, const R: usize> Resolvable for Context<F, S, L, R> {    
    
}
pub trait Bridge {}

macro_rules! impl_arithmetic {
    ($($fn:ident, $op:ident;)+) => {$(   
        impl<T: Element, const L: usize, const R: usize> ops::$op for Component<T, L> {
            pub fn $fn(self, other: Component<T, R>) -> Context<T, T, L, R> where 
                T: ops::$op,
            {   

                Context {
                    shader: Shader::Builtin($fn),
                    self,
                    other,
                    depth: Depth::Entry,
                }
            }
        }

        impl<F: Element, S: Element, const L: usize, const R: usize> ops::$op for Component<F, L> {
            pub fn $fn(self, other: Component<S, R>) -> Context<F, S, L, R> where
                F: ops::$op,
                S: ops::$op,
            {
                Context {
                    shader: Shader::Builtin($fn),
                    self,
                    other,
                    depth: Depth::Entry,
                }
            }
        }
    )+}
} 

impl_arithmetic! {
    add, Add;
}
