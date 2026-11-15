//! Rendering checks. They run on any adapter, including a software one (lavapipe) without a GPU.

use cove_render::Renderer;

#[test]
fn renders_on_any_adapter() {
    assert!(Renderer::new_headless().is_ok());
}

#[test]
fn sea_has_rough_open_sea_waves() {
    let s = Renderer::new_headless().unwrap().sea_statistics(123.0).unwrap();
    let total = s.iter().map(|c| c.height_rms * c.height_rms).sum::<f64>().sqrt();
    assert!(total > 0.5 && total < 2.0, "height RMS {total}");
}
