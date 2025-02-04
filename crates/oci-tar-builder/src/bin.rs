use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::Context;
use clap::Parser;
use oci_spec::image as spec;
use oci_tar_builder::Builder;

pub fn main() {
    let args = Args::parse();

    let out_dir;
    if let Some(out_path) = args.out_path.as_deref() {
        out_dir = PathBuf::from(out_path);
        fs::create_dir_all(&out_dir).unwrap();
    } else {
        out_dir = env::current_dir().unwrap();
    }

    let mut builder = Builder::default();

    if let Some(module_path) = args.module.as_deref() {
        let module_path = PathBuf::from(module_path);
        builder.add_layer_with_media_type(
            &module_path,
            "application/vnd.w3c.wasm.module.v1+wasm".to_string(),
        );
    }

    if let Some(components_path) = args.components.as_deref() {
        let paths = fs::read_dir(components_path).unwrap();

        for path in paths {
            let path = path.unwrap().path();
            let ext = path.extension().unwrap().to_str().unwrap();
            match ext {
                "wasm" => {
                    builder.add_layer_with_media_type(
                        &path,
                        "application/vnd.wasm.component.v1+wasm".to_string(),
                    );
                }
                "yml" | "yaml" => {
                    builder.add_layer_with_media_type(
                        &path,
                        "application/vnd.wasm.component.config.v1+yaml".to_string(),
                    );
                }
                _ => println!(
                    "Skipping Unknown file type: {:?} with extension {:?}",
                    path,
                    path.extension().unwrap()
                ),
            }
        }
    }

    let config = spec::ConfigBuilder::default().build().unwrap();

    let img = spec::ImageConfigurationBuilder::default()
        .config(config)
        .os("wasi")
        .architecture("wasm")
        .rootfs(
            spec::RootFsBuilder::default()
                .diff_ids(vec![])
                .build()
                .unwrap(),
        )
        .build()
        .context("failed to build image configuration")
        .unwrap();

    builder.add_config(img, args.repo + "/" + &args.name);

    let p = out_dir.join(args.name + ".tar");
    let f = File::create(p.clone()).unwrap();
    match builder.build(f) {
        Ok(_) => println!("Successfully created oci tar file {}", p.display()),
        Err(e) => {
            print!("Building oci tar file {} failed: {:?}", p.display(), e);
            fs::remove_file(p).unwrap_or(print!("Failed to remove temporary file"));
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    out_path: Option<String>,

    #[arg(short, long)]
    name: String,

    #[arg(short, long)]
    repo: String,

    #[arg(short, long)]
    module: Option<String>,

    #[arg(short, long)]
    components: Option<String>,
}
