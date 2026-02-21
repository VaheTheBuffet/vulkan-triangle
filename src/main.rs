mod vk_app;
mod vk_bindings;
mod math;

fn main() -> Result<(), ()> {
    let mut app = vk_app::HelloTriangleApplication::default();
    app.run();

    Ok(())
}