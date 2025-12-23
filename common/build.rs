use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from("../proto");
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("../proto/src/")
        .compile_protos(
            &[
                proto_dir.join("product.proto").to_str().unwrap(),
                proto_dir.join("user.proto").to_str().unwrap(),
                proto_dir.join("order.proto").to_str().unwrap(),
            ],
            &[proto_dir.to_str().unwrap()],
        )?;
    Ok(())
}