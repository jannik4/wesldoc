fn main() -> make::Result<()> {
    make::make("primitives", &[])?;
    make::make("math_utils", &["primitives"])?;
    make::make("pbr", &[])?;

    Ok(())
}
