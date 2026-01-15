mod vk_app;
mod vk_bindings;

fn main() -> Result<(), ()> {
    let mut app = vk_app::HelloTriangleApplication::default();
    app.run();

    Ok(())
}