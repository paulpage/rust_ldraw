use raylib::prelude::*;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write, Result};

// TODO - actually use this
type LColor = u32;

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Transformation {
    x: f32,
    y: f32,
    z: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    g: f32,
    h: f32,
    i: f32,
}

struct Polygon {
    vertices: Vec<Vec3>,
}

fn vertex_from(data: &Vec<&str>, x: usize, y: usize, z: usize) -> Vec3 {
    Vec3 {
        x: data[x].parse::<f32>().unwrap(),
        y: data[y].parse::<f32>().unwrap(),
        z: data[z].parse::<f32>().unwrap(),
    }
}

fn transform(p: &Vec3, t: &Transformation) -> Vec3 {
    Vec3 {
        x: t.a * p.x + t.b * p.y + t.c * p.z + t.x,
        y: t.d * p.x + t.e * p.y + t.f * p.z + t.y,
        z: t.g * p.x + t.h * p.y + t.i * p.z + t.z,
    }
}

fn determinant(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
    a*e*i + b*f*g + c*d*h - c*e*g - b*d*i - a*f*h
}

fn norm(p: &Polygon) -> Vec3 {
    let u = Vec3 {
        x: p.vertices[1].x - p.vertices[0].x,
        y: p.vertices[1].y - p.vertices[0].y,
        z: p.vertices[1].z - p.vertices[0].z
    };
    let v = Vec3 {
        x: p.vertices[2].x - p.vertices[0].x,
        y: p.vertices[2].y - p.vertices[0].y,
        z: p.vertices[2].z - p.vertices[0].z
    };
    Vec3 {
        x: (u.y * v.z - u.z * v.y),
        y: (u.x * v.z - u.z * v.x) * -1.0,
        z: (u.x * v.y - u.y * v.x),
    }
}

fn read_file(ldraw_directory: &str, filename: &str, inverted: bool) -> Vec<Polygon> {

    // TODO: Also allow current part's directory
    let filename = filename.to_lowercase();
    let paths: Vec<PathBuf> = vec!{
        PathBuf::new().join(ldraw_directory).join(&filename),
        PathBuf::new().join(ldraw_directory).join("parts").join(&filename),
        PathBuf::new().join(ldraw_directory).join("p").join(&filename),
        PathBuf::new().join(ldraw_directory).join("models").join(&filename),
        PathBuf::new().join(&filename),
    };
    let mut my_path = PathBuf::new();
    for path in paths {
        if path.exists() {
            my_path = path.clone();
            break;
        }
    }
    let input = File::open(my_path).unwrap();
    let input = BufReader::new(input);


    let mut polygons = Vec::new();

    let mut vertex_direction = "CCW";
    let mut invert_next = false;
    for line in input.lines() {
        let line = line.unwrap();
        // TODO: This is not correct because some whitespace (e.g. filenames)
        // should not be tokenized.
        let blocks: Vec<&str> = line.split_whitespace().collect();
        match blocks.len() {
            0..=2 => {}
            _ => {
                let command_type = blocks[0];
                let maybe_color = blocks[1];
                let data: Vec<&str> = blocks[2..].to_vec();
                match command_type {
                    "0" => {
                        if maybe_color == "BFC" {
                            if data[0] == "INVERTNEXT" {
                                invert_next = true;
                            }
                        }
                        if maybe_color == "BFC" && data.len() == 2 && data[0] == "CERTIFY" && data[1] == "CW" {
                            vertex_direction = "CW"; 
                        }
                    } // TODO 0 on the first line is the title
                    "1" => {
                        let mut invert_this = if invert_next { !inverted } else { inverted };
                        let t = Transformation {
                            x: data[0].parse::<f32>().unwrap(),
                            y: data[1].parse::<f32>().unwrap(),
                            z: data[2].parse::<f32>().unwrap(),
                            a: data[3].parse::<f32>().unwrap(),
                            b: data[4].parse::<f32>().unwrap(),
                            c: data[5].parse::<f32>().unwrap(),
                            d: data[6].parse::<f32>().unwrap(),
                            e: data[7].parse::<f32>().unwrap(),
                            f: data[8].parse::<f32>().unwrap(),
                            g: data[9].parse::<f32>().unwrap(),
                            h: data[10].parse::<f32>().unwrap(),
                            i: data[11].parse::<f32>().unwrap(),
                        };
                        if determinant(t.a, t.b, t.c, t.d, t.e, t.f, t.g, t.h, t.i) < 0.0 {
                            invert_this = !invert_this;
                        }
                        let sub_polygons = read_file(ldraw_directory, &str::replace(data[12], "\\", "/"), invert_this);
                        invert_next = false;
                        for polygon in sub_polygons {
                            let mut new_polygon = polygon;
                            new_polygon.vertices = new_polygon.vertices.iter().map(|v| transform(v, &t)).collect();
                            polygons.push(new_polygon);
                        }
                    }
                    "2" => {
                        // TODO line
                    }
                    "3" => {
                        match data.len() {
                            9 => {
                                let mut polygon = Polygon {
                                    vertices: Vec::new()
                                };
                                if (vertex_direction == "CW" && !inverted) || (vertex_direction == "CCW" && inverted) {
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                } else {
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                }
                                polygons.push(polygon);
                            }
                            _ => {} // TODO
                        }
                    }
                    "4" => {
                        match data.len() {
                            12 => {
                                let mut polygon = Polygon {
                                    vertices: Vec::new()
                                };
                                if (vertex_direction == "CW" && !inverted) || (vertex_direction == "CCW" && inverted) {
                                    polygon.vertices.push(vertex_from(&data, 9, 10, 11));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                } else {
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 9, 10, 11));
                                }
                                polygons.push(polygon);
                            }
                            _ => {} // TODO
                        }
                    }
                    "5" => {
                        // TODO optional line
                    }
                    _ => {}
                }
            }
        }
    }
    polygons
}

fn write_obj(polygons: &Vec<Polygon>, filename: &str) -> Result<()> {
    let output = OpenOptions::new().write(true).create(true).truncate(true).open(filename)?;
    let mut output = BufWriter::new(output);
    // TODO - Material doesn't seem to work, figure out why
    // writeln!(output, "mtllib untitled.mtl");

    for p in polygons {
        for v in &p.vertices {
            writeln!(output, "v {} {} {}", v.x, v.y * -1.0, v.z)?;
        }
    }
    let norms: Vec<Vec3> = polygons.clone().iter().map(|p| norm(p)).collect();
    for n in norms {
        writeln!(output, "vn {} {} {}", n.x, n.y, n.z)?;
    }
    let mut vertex_count = 1;
    // writeln!(output, "usemtl Material");
    // writeln!(output, "s off");
    for (polygon_count, p) in polygons.iter().enumerate() {
        write!(output, "f")?;
        for _ in &p.vertices {
            write!(output, " {}/{}/{}", vertex_count, vertex_count, polygon_count + 1)?;
            vertex_count += 1;
        }
        writeln!(output, "")?;
    }
    Ok(())
}

fn main() {

    let ldraw_directory = "/home/paul/Downloads/ldraw";

    let my_part_file = "car.ldr";

    let polygons = read_file(ldraw_directory, my_part_file, false);
    
    write_obj(&polygons, "test.obj");

    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("Hello, World")
        .build();

    let mut camera = Camera3D::perspective(
        Vector3::new(4.0, 2.0, 4.0), 
        Vector3::new(0.0, 1.8, 0.0), 
        Vector3::new(0.0, 1.0, 0.0), 
        60.0
    );
    rl.set_camera_mode(&camera, CameraMode::CAMERA_FIRST_PERSON);
    rl.set_target_fps(60);
    
    let model = rl.load_model(&thread, "test.obj").unwrap();

    while !rl.window_should_close() {
        rl.update_camera(&mut camera);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::DARKGREEN);
        {
            let mut d2 = d.begin_mode_3D(camera);

            d2.draw_plane(Vector3::new(0.0, 0.0, 0.0), Vector2::new(32.0, 32.0), Color::LIGHTGRAY);
            let pos = Vector3::new(1.0, 1.0, 1.0);
            let scale = 0.02;
            d2.draw_model(&model, &pos, scale, Color::WHITE);
        }
    }
}
