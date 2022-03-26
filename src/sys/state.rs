//! Copyright (c) VisualDevelopment 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

use alloc::vec::Vec;
use core::cell::UnsafeCell;

use super::{pmm::BitmapAllocator, terminal::Terminal, vmm::Pml4};
use crate::driver::acpi::Acpi;

pub static SYS_STATE: SystemState = SystemState::new();

pub struct Module {
    pub name: &'static str,
    pub data: &'static [u8],
}

#[derive(Debug)]
pub struct SystemState {
    pub pmm: UnsafeCell<spin::Once<BitmapAllocator>>,
    pub pml4: UnsafeCell<spin::Once<&'static mut Pml4>>,
    pub terminal: UnsafeCell<spin::Once<Terminal>>,
    pub acpi: UnsafeCell<spin::Once<Acpi>>,
    pub modules: UnsafeCell<spin::Once<Vec<Module>>>,
}

unsafe impl Sync for SystemState {}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            pmm: UnsafeCell::new(spin::Once::new()),
            pml4: UnsafeCell::new(spin::Once::new()),
            terminal: UnsafeCell::new(spin::Once::new()),
            acpi: UnsafeCell::new(spin::Once::new()),
            modules: UnsafeCell::new(spin::Once::new()),
        }
    }
}
