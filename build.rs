fn main() {
    relm4_icons_build::bundle_icons(
        // Name of the file that will be generated at `OUT_DIR`
        "icon_names.rs",
        // Optional app ID
        Some("com.example.myapp"),
        // Custom base resource path:
        // * defaults to `/com/example/myapp` in this case if not specified explicitly
        // * or `/org/relm4` if app ID was not specified either
        None::<&str>,
        // Directory with custom icons (if any)
        Some("icons"),
        // List of icons to include
        ["tag"],
    );

    // Compile GSettings schemas for local development
    println!("cargo:rerun-if-changed=data/com.marca.app.gschema.xml");
    if let Err(e) = std::process::Command::new("glib-compile-schemas")
        .arg("data")
        .status()
    {
        println!("cargo:warning=Failed to compile GSettings schemas: {}", e);
    }
}
