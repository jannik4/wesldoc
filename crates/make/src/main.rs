fn main() -> make::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    make::make("primitives", &[])?;
    make::make("math_utils", &["primitives"])?;
    make::make("pbr", &[])?;

    Ok(())
}
