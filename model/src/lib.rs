#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(macro_metavar_expr)]

pub mod mitigations;
mod model;
mod model_unified;
pub mod parameters;
#[cfg(test)]
mod snapshot;
mod utils;

// Temporary legacy stubs — consumed by src/App.vue via the vite wasm-pack plugin.
// Removed in the final step of the port once SEIRModelUnified lands.
use cfasim_model::{ModelOutput, model_outputs};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn double(n: i32) -> i32 {
    n * 2
}

#[wasm_bindgen]
pub fn simulate(steps: u32, rate: f64) -> JsValue {
    let time: Vec<f64> = (0..steps).map(|i| i as f64).collect();
    let values: Vec<f64> = (0..steps).map(|i| rate * i as f64).collect();

    let series = ModelOutput::new(steps as usize)
        .add_f64("time", time)
        .add_f64("values", values);

    model_outputs([("series", series)])
}
