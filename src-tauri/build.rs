fn main() {
    println!("cargo:rerun-if-changed=icons");

    #[cfg(windows)]
    {
        let attrs = tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        tauri_build::try_build(attrs).expect("failed to run tauri-build");

        let out_dir = std::path::PathBuf::from(
            std::env::var("OUT_DIR").expect("OUT_DIR must be set in build.rs"),
        );
        let manifest_rc = out_dir.join("common_controls.rc");
        std::fs::write(
            &manifest_rc,
            concat!(
                "#pragma code_page(65001)\n",
                "1 24\n",
                "{\n",
                "\"<assembly xmlns=\"\"urn:schemas-microsoft-com:asm.v1\"\" manifestVersion=\"\"1.0\"\"><dependency><dependentAssembly><assemblyIdentity type=\"\"win32\"\" name=\"\"Microsoft.Windows.Common-Controls\"\" version=\"\"6.0.0.0\"\" processorArchitecture=\"\"*\"\" publicKeyToken=\"\"6595b64144ccf1df\"\" language=\"\"*\"\" /></dependentAssembly></dependency></assembly>\"\n",
                "}\n",
            ),
        )
        .expect("failed to write common_controls.rc");

        embed_resource::compile_for_everything(&manifest_rc, embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }

    #[cfg(not(windows))]
    tauri_build::build();
}
