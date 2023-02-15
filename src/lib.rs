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

pub struct Component<T: Element, const N: usize> {
    identity: Identity,
    local: [T; N], 
}

pub struct Context<T: Element, const N: usize> {
    depth: Depth,
    shader: Shader,
    lhs: Component<T, N>, 
    rhs: Component<T, N>,
}

pub trait Bridge {}

pub trait Resolvable {
    type Output;

    fn resolve(self) -> Self::Output;
}

impl<T: Element, const N: usize> Resolvable for [T; N] {
    type Output = Component<T, N>;

    fn resolve(self) -> Self::Output {
        Component { 
            identity: unsafe {
                static mut ops: u32 = 0;
                ops += 1;
            
                match ops {
                    1 => Identity::Unique, 
                    _ => {
                        Identity::Shared(ops)
                    }
                }
            },
            local: self,
        }
    }
} 

macro_rules! impl_arithmetic {
    ($($fn:ident, $op:ident;)+) => {$(   

        impl<T: Element, const N: usize> ops::$op for Component<T, N> {        
            type Output = Context<T, N>;

            fn $fn(self, other: Component<T, N>) -> Self::Output where {  
                Context {
                    depth: Depth::Entry,
                    shader: Shader::Builtin("$fn"),
                    lhs: self,
                    rhs: other,
                }
            }
        }

    )+}
} 

impl_arithmetic! {
    add, Add;
    sub, Sub;
    mul, Mul;
    div, Div;
}
