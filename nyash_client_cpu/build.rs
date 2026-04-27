

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

    Ok(())
}