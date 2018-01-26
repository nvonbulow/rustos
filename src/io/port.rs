use core::marker::{Sized, PhantomData};
use x86_64::instructions::port::*;

pub trait InOut where Self: Sized {
    unsafe fn port_read(port: u16) -> Self;
    unsafe fn port_write(port: u16, val: Self);
    unsafe fn port_write_buffer(port: u16, buf: &[Self]);
}

impl InOut for u8 {
    unsafe fn port_read(port: u16) -> Self {
        inb(port)
    }

    unsafe fn port_write(port: u16, val: Self) {
        outb(port, val)
    }

    unsafe fn port_write_buffer(port: u16, buf: &[Self]) {
        outsb(port, buf)
    }
}

impl InOut for u16 {
    unsafe fn port_read(port: u16) -> Self {
        inw(port)
    }

    unsafe fn port_write(port: u16, val: Self) {
        outw(port, val)
    }

    unsafe fn port_write_buffer(port: u16, buf: &[Self]) {
        outsw(port, buf)
    }
}

impl InOut for u32 {
    unsafe fn port_read(port: u16) -> Self {
        inl(port)
    }

    unsafe fn port_write(port: u16, val: Self) {
        outl(port, val)
    }

    unsafe fn port_write_buffer(port: u16, buf: &[Self]) {
        outsl(port, buf)
    }
}

pub struct Port<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port {
            port,
            phantom: PhantomData
        }
    }

    pub fn read(&mut self) -> T {
        unsafe {
            T::port_read(self.port)
        }
    }

    pub fn write(&mut self, val: T) {
        unsafe {
            T::port_write(self.port, val)
        }
    }

    pub fn write_buffer(&mut self, buf: &[T]) {
        unsafe {
            T::port_write_buffer(self.port, buf)
        }
    }
}

pub struct UnsafePort<T> {
    port: u16,
    phantom: PhantomData<T>
}

impl<T: InOut> UnsafePort<T> {
    pub unsafe fn new(port: u16) -> UnsafePort<T> {
        UnsafePort {
            port,
            phantom: PhantomData
        }
    }

    pub unsafe fn read(&mut self) -> T {
        T::port_read(self.port)
    }

    pub unsafe fn write(&mut self, val: T) {
        T::port_write(self.port, val)
    }

    pub unsafe fn write_buffer(&mut self, buf: &[T]) {
        T::port_write_buffer(self.port, buf)
    }
}
