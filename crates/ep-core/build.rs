use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(&["../../proto/ep/v1/conversation.proto"], &["../../proto"])?;
    Ok(())
}
