// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use clap::{Args, Subcommand};
use colored::*;
use rand::Rng;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct GenerateArgs {
    #[command(subcommand)]
    pub command: GenerateCommands,
}

#[derive(Subcommand)]
pub enum GenerateCommands {
    /// Generate synthetic engineering files for testing
    Files {
        /// Number of each file type to generate
        #[arg(short, long, default_value = "3")]
        count: u32,

        /// Output directory
        #[arg(short, long, default_value = "./generated-files")]
        output: PathBuf,

        /// Complexity level: simple, medium, complex
        #[arg(long, default_value = "medium")]
        complexity: String,
    },
}

pub async fn execute(args: GenerateArgs) -> anyhow::Result<()> {
    match args.command {
        GenerateCommands::Files {
            count,
            output,
            complexity,
        } => generate_files(count, output, &complexity).await,
    }
}

async fn generate_files(count: u32, output: PathBuf, complexity: &str) -> anyhow::Result<()> {
    let settings = match complexity {
        "simple" => ComplexitySettings {
            vertices: 8,
            elements: 50,
            points: 1000,
        },
        "complex" => ComplexitySettings {
            vertices: 200,
            elements: 1000,
            points: 100000,
        },
        _ => ComplexitySettings {
            vertices: 50,
            elements: 200,
            points: 10000,
        }, // medium
    };

    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║         Engineering Files Generator (Rust)                   ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!("\nOutput:     {}", output.display());
    println!("Count:      {} of each type", count);
    println!(
        "Complexity: {} (vertices: {}, elements: {})",
        complexity, settings.vertices, settings.elements
    );

    // Create output directory
    fs::create_dir_all(&output)?;

    let mut stats = Stats::default();
    let mut total_bytes: u64 = 0;

    // Generate OBJ files
    println!(
        "\n{}",
        "[1/6] Generating OBJ files (3D mesh geometry)...".yellow()
    );
    for i in 1..=count {
        let (size, mtl_size) = generate_obj(&output, i, &settings)?;
        total_bytes += size + mtl_size;
        stats.obj += 1;
        println!(
            "  {} building-model-{}.obj ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Generate DXF files
    println!(
        "\n{}",
        "[2/6] Generating DXF files (AutoCAD drawings)...".yellow()
    );
    for i in 1..=count {
        let size = generate_dxf(&output, i)?;
        total_bytes += size;
        stats.dxf += 1;
        println!(
            "  {} floorplan-{}.dxf ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Generate STL files
    println!(
        "\n{}",
        "[3/6] Generating STL files (3D printing meshes)...".yellow()
    );
    for i in 1..=count {
        let size = generate_stl(&output, i)?;
        total_bytes += size;
        stats.stl += 1;
        println!(
            "  {} part-{}.stl ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Generate IFC files
    println!(
        "\n{}",
        "[4/6] Generating IFC files (BIM models)...".yellow()
    );
    for i in 1..=count {
        let size = generate_ifc(&output, i, &settings)?;
        total_bytes += size;
        stats.ifc += 1;
        println!(
            "  {} building-{}.ifc ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Generate JSON metadata
    println!("\n{}", "[5/6] Generating JSON metadata files...".yellow());
    for i in 1..=count {
        let size = generate_json(&output, i, &settings)?;
        total_bytes += size;
        stats.json += 1;
        println!(
            "  {} project-{}-metadata.json ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Generate point cloud files
    println!("\n{}", "[6/6] Generating point cloud files...".yellow());
    for i in 1..=count {
        let size = generate_xyz(&output, i, &settings)?;
        total_bytes += size;
        stats.xyz += 1;
        println!(
            "  {} scan-{}.xyz ({:.1} KB)",
            "✓".green(),
            i,
            size as f64 / 1024.0
        );
    }

    // Summary
    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║                    Generation Complete                        ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    println!("\n  Output: {}", fs::canonicalize(&output)?.display());
    println!("\n  Files Generated:");
    println!("    OBJ (3D mesh):     {} files", stats.obj);
    println!("    DXF (AutoCAD):     {} files", stats.dxf);
    println!("    STL (3D print):    {} files", stats.stl);
    println!("    IFC (BIM):         {} files", stats.ifc);
    println!("    JSON (metadata):   {} files", stats.json);
    println!("    XYZ (point cloud): {} files", stats.xyz);
    println!("    ────────────────────────────");
    let total = stats.obj + stats.dxf + stats.stl + stats.ifc + stats.json + stats.xyz;
    println!(
        "    Total:             {} files ({:.1} KB)",
        total,
        total_bytes as f64 / 1024.0
    );

    println!("\n  {}", "Compatible with APS Translation:".green());
    println!("    ✓ OBJ → SVF/SVF2 viewer format");
    println!("    ✓ DXF → SVF/SVF2 viewer format");
    println!("    ✓ STL → SVF/SVF2 viewer format");
    println!("    ✓ IFC → SVF/SVF2 viewer format");

    println!("\n{}", "=== Generation Complete ===".cyan());

    Ok(())
}

struct ComplexitySettings {
    vertices: u32,
    elements: u32,
    points: u32,
}

#[derive(Default)]
struct Stats {
    obj: u32,
    dxf: u32,
    stl: u32,
    ifc: u32,
    json: u32,
    xyz: u32,
}

fn generate_obj(
    output: &Path,
    index: u32,
    _settings: &ComplexitySettings,
) -> anyhow::Result<(u64, u64)> {
    let obj_path = output.join(format!("building-model-{}.obj", index));
    let mtl_path = output.join(format!("building-model-{}.mtl", index));

    let mut obj_content = format!(
        "# APS Demo - Building Model {}\n# Generated by raps\n\nmtllib building-model-{}.mtl\n\n",
        index, index
    );

    let mut vertex_offset = 0u32;

    // Building components
    let components = vec![
        ("Foundation", 20.0, 1.0, 15.0, 0.0, -0.5, 0.0),
        ("Floor1", 18.0, 3.0, 13.0, 0.0, 2.0, 0.0),
        ("Floor2", 18.0, 3.0, 13.0, 0.0, 5.5, 0.0),
        ("Roof", 20.0, 0.5, 15.0, 0.0, 7.5, 0.0),
    ];

    for (name, w, h, d, cx, cy, cz) in components {
        obj_content.push_str(&format!("\no {}\nusemtl {}_material\n", name, name));

        // Box vertices
        let hw = w / 2.0;
        let hh = h / 2.0;
        let hd = d / 2.0;
        let verts = vec![
            (cx - hw, cy - hh, cz + hd),
            (cx + hw, cy - hh, cz + hd),
            (cx + hw, cy + hh, cz + hd),
            (cx - hw, cy + hh, cz + hd),
            (cx - hw, cy - hh, cz - hd),
            (cx + hw, cy - hh, cz - hd),
            (cx + hw, cy + hh, cz - hd),
            (cx - hw, cy + hh, cz - hd),
        ];

        for (x, y, z) in &verts {
            obj_content.push_str(&format!("v {:.6} {:.6} {:.6}\n", x, y, z));
        }

        // Faces (1-indexed, offset by vertex_offset)
        let o = vertex_offset + 1;
        obj_content.push_str(&format!("f {} {} {} {}\n", o, o + 1, o + 2, o + 3));
        obj_content.push_str(&format!("f {} {} {} {}\n", o + 7, o + 6, o + 5, o + 4));
        obj_content.push_str(&format!("f {} {} {} {}\n", o + 3, o + 2, o + 6, o + 7));
        obj_content.push_str(&format!("f {} {} {} {}\n", o + 4, o + 5, o + 1, o));
        obj_content.push_str(&format!("f {} {} {} {}\n", o + 1, o + 5, o + 6, o + 2));
        obj_content.push_str(&format!("f {} {} {} {}\n", o + 4, o, o + 3, o + 7));

        vertex_offset += 8;
    }

    // Write OBJ file
    let mut obj_file = File::create(&obj_path)?;
    obj_file.write_all(obj_content.as_bytes())?;

    // Generate MTL file
    let mtl_content = format!(
        "# Material Library for building-model-{}.obj\n\n\
        newmtl Foundation_material\nKd 0.5 0.5 0.5\nKa 0.1 0.1 0.1\n\n\
        newmtl Floor1_material\nKd 0.8 0.8 0.7\nKa 0.1 0.1 0.1\n\n\
        newmtl Floor2_material\nKd 0.8 0.8 0.7\nKa 0.1 0.1 0.1\n\n\
        newmtl Roof_material\nKd 0.3 0.3 0.4\nKa 0.1 0.1 0.1\n",
        index
    );

    let mut mtl_file = File::create(&mtl_path)?;
    mtl_file.write_all(mtl_content.as_bytes())?;

    Ok((obj_path.metadata()?.len(), mtl_path.metadata()?.len()))
}

fn generate_dxf(output: &Path, index: u32) -> anyhow::Result<u64> {
    let mut rng = rand::thread_rng();
    let width: f64 = rng.gen_range(20.0..40.0);
    let height: f64 = rng.gen_range(15.0..30.0);
    let rooms: u32 = rng.gen_range(3..8);

    let path = output.join(format!("floorplan-{}.dxf", index));

    let mut content = String::from(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n"
    );

    // Outer walls
    let walls = vec![
        (0.0, 0.0, width, 0.0),
        (width, 0.0, width, height),
        (width, height, 0.0, height),
        (0.0, height, 0.0, 0.0),
    ];

    for (x1, y1, x2, y2) in walls {
        content.push_str(&format!(
            "0\nLINE\n8\nWalls\n10\n{:.1}\n20\n{:.1}\n30\n0.0\n11\n{:.1}\n21\n{:.1}\n31\n0.0\n",
            x1, y1, x2, y2
        ));
    }

    // Room divisions
    for r in 1..rooms {
        let div_x = width * r as f64 / rooms as f64;
        content.push_str(&format!(
            "0\nLINE\n8\nInterior_Walls\n10\n{:.1}\n20\n0.0\n30\n0.0\n11\n{:.1}\n21\n{:.1}\n31\n0.0\n",
            div_x, div_x, height
        ));
    }

    // Doors
    for d in 0..rooms {
        let door_x = width * (d as f64 + 0.5) / rooms as f64;
        content.push_str(&format!(
            "0\nCIRCLE\n8\nDoors\n10\n{:.1}\n20\n0.5\n30\n0.0\n40\n0.8\n",
            door_x
        ));
    }

    // Dimension text
    content.push_str(&format!(
        "0\nTEXT\n8\nDimensions\n10\n{:.1}\n20\n-2.0\n30\n0.0\n40\n1.0\n1\n{:.0}m x {:.0}m\n",
        width / 2.0,
        width,
        height
    ));

    content.push_str("0\nENDSEC\n0\nEOF\n");

    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;

    Ok(path.metadata()?.len())
}

fn generate_stl(output: &Path, index: u32) -> anyhow::Result<u64> {
    let mut rng = rand::thread_rng();
    let scale: f64 = rng.gen_range(10.0..30.0);

    let path = output.join(format!("part-{}.stl", index));

    let content = format!(
        "solid Part_{}\n\
          facet normal 0 0 1\n    outer loop\n      vertex 0 0 {s}\n      vertex {s} 0 {s}\n      vertex {s} {s} {s}\n    endloop\n  endfacet\n\
          facet normal 0 0 1\n    outer loop\n      vertex 0 0 {s}\n      vertex {s} {s} {s}\n      vertex 0 {s} {s}\n    endloop\n  endfacet\n\
          facet normal 0 0 -1\n    outer loop\n      vertex 0 0 0\n      vertex {s} {s} 0\n      vertex {s} 0 0\n    endloop\n  endfacet\n\
          facet normal 0 0 -1\n    outer loop\n      vertex 0 0 0\n      vertex 0 {s} 0\n      vertex {s} {s} 0\n    endloop\n  endfacet\n\
          facet normal 0 -1 0\n    outer loop\n      vertex 0 0 0\n      vertex {s} 0 0\n      vertex {s} 0 {s}\n    endloop\n  endfacet\n\
          facet normal 0 -1 0\n    outer loop\n      vertex 0 0 0\n      vertex {s} 0 {s}\n      vertex 0 0 {s}\n    endloop\n  endfacet\n\
          facet normal 0 1 0\n    outer loop\n      vertex 0 {s} 0\n      vertex {s} {s} {s}\n      vertex {s} {s} 0\n    endloop\n  endfacet\n\
          facet normal 0 1 0\n    outer loop\n      vertex 0 {s} 0\n      vertex 0 {s} {s}\n      vertex {s} {s} {s}\n    endloop\n  endfacet\n\
          facet normal -1 0 0\n    outer loop\n      vertex 0 0 0\n      vertex 0 {s} {s}\n      vertex 0 {s} 0\n    endloop\n  endfacet\n\
          facet normal -1 0 0\n    outer loop\n      vertex 0 0 0\n      vertex 0 0 {s}\n      vertex 0 {s} {s}\n    endloop\n  endfacet\n\
          facet normal 1 0 0\n    outer loop\n      vertex {s} 0 0\n      vertex {s} {s} 0\n      vertex {s} {s} {s}\n    endloop\n  endfacet\n\
          facet normal 1 0 0\n    outer loop\n      vertex {s} 0 0\n      vertex {s} {s} {s}\n      vertex {s} 0 {s}\n    endloop\n  endfacet\n\
        endsolid Part_{}\n",
        index, index, s = scale
    );

    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;

    Ok(path.metadata()?.len())
}

fn generate_ifc(output: &Path, index: u32, settings: &ComplexitySettings) -> anyhow::Result<u64> {
    let mut rng = rand::thread_rng();
    let path = output.join(format!("building-{}.ifc", index));

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let project_guid = generate_ifc_guid();
    let site_guid = generate_ifc_guid();
    let building_guid = generate_ifc_guid();

    let mut content = format!(
        "ISO-10303-21;\n\
        HEADER;\n\
        FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');\n\
        FILE_NAME('building-{}.ifc','{}',('RAPS Generator'),('Demo Organization'),'IFC4','raps','');\n\
        FILE_SCHEMA(('IFC4'));\n\
        ENDSEC;\n\n\
        DATA;\n\
        #1=IFCPROJECT('{}',#2,'Building Project {}','Demo building model',`$,`$,`$,(#7),#11);\n\
        #2=IFCOWNERHISTORY(#3,#6,`$,.NOCHANGE.,`$,`$,`$,1234567890);\n\
        #3=IFCPERSONANDORGANIZATION(#4,#5,`$);\n\
        #4=IFCPERSON(`$,'Generator','CLI',`$,`$,`$,`$,`$);\n\
        #5=IFCORGANIZATION(`$,'Demo Corp','Demo Organization',`$,`$);\n\
        #6=IFCAPPLICATION(#5,'1.0','APS CLI Generator','APSCLI');\n\
        #7=IFCGEOMETRICREPRESENTATIONCONTEXT(`$,'Model',3,1.E-05,#8,#9);\n\
        #8=IFCAXIS2PLACEMENT3D(#10,`$,`$);\n\
        #9=IFCDIRECTION((0.,1.,0.));\n\
        #10=IFCCARTESIANPOINT((0.,0.,0.));\n\
        #11=IFCUNITASSIGNMENT((#12,#13,#14,#15));\n\
        #12=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n\
        #13=IFCSIUNIT(*,.AREAUNIT.,`$,.SQUARE_METRE.);\n\
        #14=IFCSIUNIT(*,.VOLUMEUNIT.,`$,.CUBIC_METRE.);\n\
        #15=IFCSIUNIT(*,.PLANEANGLEUNIT.,`$,.RADIAN.);\n\n\
        #20=IFCSITE('{}',#2,'Site','Building site',`$,#21,`$,`$,.ELEMENT.,`$,`$,`$,`$,`$);\n\
        #21=IFCLOCALPLACEMENT(`$,#8);\n\n\
        #30=IFCBUILDING('{}',#2,'Building {}','Main building',`$,#31,`$,`$,.ELEMENT.,`$,`$,`$);\n\
        #31=IFCLOCALPLACEMENT(#21,#8);\n\n\
        #40=IFCRELAGGREGATES('{}',#2,`$,`$,#1,(#20));\n\
        #41=IFCRELAGGREGATES('{}',#2,`$,`$,#20,(#30));\n\n",
        index, timestamp, project_guid, index, site_guid, building_guid, index,
        generate_ifc_guid(), generate_ifc_guid()
    );

    // Add storeys and elements
    let storey_count = rng.gen_range(2..5);
    let mut entity_id = 100u32;

    let ifc_categories = [
        "IfcWall",
        "IfcDoor",
        "IfcWindow",
        "IfcSlab",
        "IfcColumn",
        "IfcBeam",
    ];

    for s in 0..storey_count {
        let elevation = s * 3000;
        content.push_str(&format!(
            "#{}=IFCBUILDINGSTOREY('{}',#2,'Level {}','Storey at {}mm',`$,#{},`$,`$,.ELEMENT.,{}.0);\n\
            #{}=IFCLOCALPLACEMENT(#31,#8);\n",
            entity_id, generate_ifc_guid(), s + 1, elevation, entity_id + 1, elevation, entity_id + 1
        ));
        entity_id += 2;
    }

    // Add elements
    let _elements_per_storey = settings.elements / storey_count;
    for _ in 0..settings.elements {
        let cat = ifc_categories[rng.gen_range(0..ifc_categories.len())];
        content.push_str(&format!(
            "#{}={}('{}',#2,'{}_{}',' ',`$,`$,`$,`$);\n",
            entity_id,
            cat,
            generate_ifc_guid(),
            cat,
            entity_id
        ));
        entity_id += 1;
    }

    content.push_str("ENDSEC;\nEND-ISO-10303-21;\n");

    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;

    Ok(path.metadata()?.len())
}

fn generate_json(output: &Path, index: u32, settings: &ComplexitySettings) -> anyhow::Result<u64> {
    let mut rng = rand::thread_rng();
    let path = output.join(format!("project-{}-metadata.json", index));

    let categories = [
        "Walls", "Doors", "Windows", "Floors", "Ceilings", "Columns", "Beams",
    ];
    let levels = ["Basement", "Level 1", "Level 2", "Level 3", "Roof"];
    let materials = ["Concrete", "Steel", "Wood", "Glass", "Aluminum", "Brick"];

    let mut elements = Vec::new();
    let mut total_area = 0.0f64;
    let mut total_volume = 0.0f64;

    for i in 1..=settings.elements {
        let area: f64 = rng.gen_range(1.0..500.0);
        let volume: f64 = rng.gen_range(1.0..200.0);
        total_area += area;
        total_volume += volume;

        elements.push(serde_json::json!({
            "dbId": i,
            "externalId": uuid::Uuid::new_v4().to_string(),
            "name": format!("{}_{}", categories[rng.gen_range(0..categories.len())], i),
            "category": categories[rng.gen_range(0..categories.len())],
            "level": levels[rng.gen_range(0..levels.len())],
            "material": materials[rng.gen_range(0..materials.len())],
            "geometry": {
                "area": (area * 100.0).round() / 100.0,
                "volume": (volume * 100.0).round() / 100.0,
            },
            "visible": rng.gen_bool(0.9),
        }));
    }

    let metadata = serde_json::json!({
        "projectInfo": {
            "id": uuid::Uuid::new_v4().to_string(),
            "name": format!("Demo Project {}", index),
            "number": format!("PRJ-{:04}", rng.gen_range(1000..9999)),
        },
        "modelInfo": {
            "version": format!("2024.{}", index),
            "units": "millimeters",
        },
        "statistics": {
            "totalElements": settings.elements,
            "totalArea": (total_area * 100.0).round() / 100.0,
            "totalVolume": (total_volume * 100.0).round() / 100.0,
        },
        "elements": elements,
    });

    let mut file = File::create(&path)?;
    file.write_all(serde_json::to_string_pretty(&metadata)?.as_bytes())?;

    Ok(path.metadata()?.len())
}

fn generate_xyz(output: &Path, index: u32, settings: &ComplexitySettings) -> anyhow::Result<u64> {
    let mut rng = rand::thread_rng();
    let path = output.join(format!("scan-{}.xyz", index));

    let mut content = format!(
        "# XYZ Point Cloud - Scan {}\n# Points: {}\n# Format: X Y Z R G B Intensity\n",
        index, settings.points
    );

    for _ in 0..settings.points {
        // Generate points on building surfaces
        let surface = rng.gen_range(0..6);
        let (x, y, z) = match surface {
            0 => (rng.gen_range(-20.0..20.0), -10.0, rng.gen_range(0.0..10.0)),
            1 => (rng.gen_range(-20.0..20.0), 10.0, rng.gen_range(0.0..10.0)),
            2 => (-20.0, rng.gen_range(-10.0..10.0), rng.gen_range(0.0..10.0)),
            3 => (20.0, rng.gen_range(-10.0..10.0), rng.gen_range(0.0..10.0)),
            4 => (rng.gen_range(-20.0..20.0), rng.gen_range(-10.0..10.0), 0.0),
            _ => (rng.gen_range(-20.0..20.0), rng.gen_range(-10.0..10.0), 10.0),
        };

        // Add noise
        let x = x + rng.gen_range(-0.5..0.5);
        let y = y + rng.gen_range(-0.5..0.5);
        let z = z + rng.gen_range(-0.5..0.5);

        let r: u8 = rng.gen_range(100..200);
        let g: u8 = rng.gen_range(100..200);
        let b: u8 = rng.gen_range(100..200);
        let intensity: f64 = rng.gen_range(0.5..1.0);

        content.push_str(&format!(
            "{:.3} {:.3} {:.3} {} {} {} {:.2}\n",
            x, y, z, r, g, b, intensity
        ));
    }

    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;

    Ok(path.metadata()?.len())
}

fn generate_ifc_guid() -> String {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let mut result = String::with_capacity(22);

    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

    for i in 0..22 {
        let idx = (bytes[i % 16] as usize + i) % 64;
        result.push(CHARS[idx] as char);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ifc_guid_format() {
        let guid = generate_ifc_guid();

        // IFC GUIDs should be exactly 22 characters
        assert_eq!(guid.len(), 22);

        // Should only contain valid IFC GUID characters
        const VALID_CHARS: &str =
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
        for ch in guid.chars() {
            assert!(
                VALID_CHARS.contains(ch),
                "Invalid character in IFC GUID: {}",
                ch
            );
        }
    }

    #[test]
    fn test_generate_ifc_guid_uniqueness() {
        // Generate multiple GUIDs and verify they're different
        let guid1 = generate_ifc_guid();
        let guid2 = generate_ifc_guid();
        let guid3 = generate_ifc_guid();

        assert_ne!(guid1, guid2);
        assert_ne!(guid2, guid3);
        assert_ne!(guid1, guid3);
    }

    #[test]
    fn test_generate_ifc_guid_multiple_calls() {
        // Generate 100 GUIDs to ensure they're all valid
        for _ in 0..100 {
            let guid = generate_ifc_guid();
            assert_eq!(guid.len(), 22);
        }
    }
}
