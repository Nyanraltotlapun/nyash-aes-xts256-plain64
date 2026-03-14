use std::env;
use std::io::Write;
use std::path::Path;
use ocl_include;
use flate2::write::GzEncoder;
use flate2::Compression;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //compile gRPC
    tonic_prost_build::compile_protos("proto/nyash.proto")?;


    //******************* compile ocl into spirv64 *********
    // let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // let ocl_src_dir = Path::new(&manifest_dir).join("src/open_cl");
    // let spirv_build_script = ocl_src_dir.clone().join("build_spirv.sh");

    
    // let out_dir = env::var("OUT_DIR").unwrap();
    // let spirv_bin_path = Path::new(&out_dir).join("nyash_aes_xts256_plain.spv");
    // let str_spirv_bit_p = spirv_bin_path.to_str().expect("Error converting spirv out path to str!");
    // let _output = Command::new(spirv_build_script)
    //     .current_dir(ocl_src_dir)
    //     .arg(str_spirv_bit_p)
    //     .output()
    //     .expect("Failed to execute spirv build script!");

    // ***************** concat ocl sources to one file and zip it **********************
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ocl_src_dir = Path::new(&manifest_dir).join("src/open_cl");
    let ocl_src_file = ocl_src_dir.clone().join("nyash_aes_xts256_plain.cl");

    let ocl_parser = ocl_include::Parser::builder()
        .add_source(
            ocl_include::source::Fs::builder()
                .include_dir(ocl_src_dir)
                .expect("Error adding ocl include dir!")
                .build(),
        )
        .build();
    println!("Concatenating and compressing ocl sources...");
    let ocl_node = ocl_parser.parse(&ocl_src_file).expect("Error parsing ocl source!");
    let full_src = ocl_node.collect().0;
    let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::best());

    gz_encoder.write_all(full_src.as_bytes()).expect("Error compressing src file!");
    
    let compressed_bytes = gz_encoder.finish().expect("Error compressing src!");

    let out_dir = env::var("OUT_DIR").unwrap();
    let ocl_concat_src_path = Path::new(&out_dir).join("nyash_aes_full.cl.gz");
    std::fs::write(ocl_concat_src_path, compressed_bytes).expect("Error writing compressed src file!");

    Ok(())
}