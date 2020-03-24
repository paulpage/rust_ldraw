use cgmath::prelude::*;
use cgmath::{Matrix4, Point3, Vector3};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Result, Write};
use std::path::PathBuf;
use std::time::Instant;

// TODO - actually use this
// type LColor = u32;

pub struct Polygon {
    pub points: Vec<Point3<f32>>,
}

fn point_from(data: &[&str], x: usize, y: usize, z: usize) -> Point3<f32> {
    Point3 {
        x: data[x].parse::<f32>().unwrap(),
        y: data[y].parse::<f32>().unwrap(),
        z: data[z].parse::<f32>().unwrap(),
    }
}

pub fn norm(p: &Polygon) -> Vector3<f32> {
    let u = Vector3 {
        x: p.points[1].x - p.points[0].x,
        y: p.points[1].y - p.points[0].y,
        z: p.points[1].z - p.points[0].z,
    };
    let v = Vector3 {
        x: p.points[2].x - p.points[0].x,
        y: p.points[2].y - p.points[0].y,
        z: p.points[2].z - p.points[0].z,
    };
    Vector3 {
        x: (u.y * v.z - u.z * v.y),
        y: (u.x * v.z - u.z * v.x) * -1.0,
        z: (u.x * v.y - u.y * v.x),
    }
}

pub fn read_file(ldraw_directory: &str, filename: &str, inverted: bool) -> Vec<Polygon> {
    // TODO: Also allow current part's directory
    let filename = filename.to_lowercase();
    let paths: Vec<PathBuf> = vec![
        PathBuf::new().join(ldraw_directory).join(&filename),
        PathBuf::new()
            .join(ldraw_directory)
            .join("parts")
            .join(&filename),
        PathBuf::new()
            .join(ldraw_directory)
            .join("p")
            .join(&filename),
        PathBuf::new()
            .join(ldraw_directory)
            .join("models")
            .join(&filename),
        PathBuf::new().join(&filename),
    ];
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
                        if maybe_color == "BFC" && data[0] == "INVERTNEXT" {
                            invert_next = true;
                        }
                        if maybe_color == "BFC"
                            && data.len() == 2
                            && data[0] == "CERTIFY"
                            && data[1] == "CW"
                        {
                            vertex_direction = "CW";
                        }
                    } // TODO 0 on the first line is the title
                    "1" => {
                        let mut invert_this = if invert_next { !inverted } else { inverted };
                        let t = Matrix4::new(
                            data[3].parse::<f32>().unwrap(), //a
                            data[6].parse::<f32>().unwrap(), //d
                            data[9].parse::<f32>().unwrap(), //g
                            0.0,
                            data[4].parse::<f32>().unwrap(),  //b
                            data[7].parse::<f32>().unwrap(),  //e
                            data[10].parse::<f32>().unwrap(), //h
                            0.0,
                            data[5].parse::<f32>().unwrap(),  //c
                            data[8].parse::<f32>().unwrap(),  //f
                            data[11].parse::<f32>().unwrap(), //i
                            0.0,
                            data[0].parse::<f32>().unwrap(), //x
                            data[1].parse::<f32>().unwrap(), //y
                            data[2].parse::<f32>().unwrap(), //z
                            1.0,
                        );
                        if t.determinant() < 0.0 {
                            invert_this = !invert_this;
                        }
                        let sub_polygons = read_file(
                            ldraw_directory,
                            &str::replace(data[12], "\\", "/"),
                            invert_this,
                        );
                        invert_next = false;
                        for polygon in sub_polygons {
                            let mut new_polygon = polygon;
                            new_polygon.points = new_polygon
                                .points
                                .iter()
                                .map(|p| t.transform_point(*p))
                                .collect();
                            polygons.push(new_polygon);
                        }
                    }
                    "2" => {
                        // TODO line
                    }
                    "3" => {
                        if data.len() == 9 {
                            let mut polygon = Polygon { points: Vec::new() };
                            if (vertex_direction == "CW" && !inverted)
                                || (vertex_direction == "CCW" && inverted)
                            {
                                polygon.points.push(point_from(&data, 6, 7, 8));
                                polygon.points.push(point_from(&data, 3, 4, 5));
                                polygon.points.push(point_from(&data, 0, 1, 2));
                            } else {
                                polygon.points.push(point_from(&data, 0, 1, 2));
                                polygon.points.push(point_from(&data, 3, 4, 5));
                                polygon.points.push(point_from(&data, 6, 7, 8));
                            }
                            polygons.push(polygon);
                        }
                    }
                    "4" => {
                        if data.len() == 12 {
                            let mut polygon = Polygon { points: Vec::new() };
                            let mut polygon2 = Polygon { points: Vec::new() };
                            if (vertex_direction == "CW" && !inverted)
                                || (vertex_direction == "CCW" && inverted)
                            {
                                polygon.points.push(point_from(&data, 9, 10, 11));
                                polygon.points.push(point_from(&data, 6, 7, 8));
                                polygon.points.push(point_from(&data, 3, 4, 5));
                                polygon2.points.push(point_from(&data, 3, 4, 5));
                                polygon2.points.push(point_from(&data, 0, 1, 2));
                                polygon2.points.push(point_from(&data, 9, 10, 11));
                            } else {
                                polygon.points.push(point_from(&data, 0, 1, 2));
                                polygon.points.push(point_from(&data, 3, 4, 5));
                                polygon.points.push(point_from(&data, 6, 7, 8));
                                polygon2.points.push(point_from(&data, 6, 7, 8));
                                polygon2.points.push(point_from(&data, 9, 10, 11));
                                polygon2.points.push(point_from(&data, 0, 1, 2));
                            }
                            polygons.push(polygon);
                            polygons.push(polygon2);
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

pub fn write_obj(polygons: &[Polygon], filename: &str) -> Result<()> {
    let start = Instant::now();
    let output = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(filename)?;
    let mut output = BufWriter::new(output);
    // writeln!(output, "mtllib test.mtl")?;

    for p in polygons {
        for v in &p.points {
            writeln!(output, "v {} {} {}", v.x, v.y * -1.0, v.z)?;
        }
    }
    let norms: Vec<Vector3<f32>> = polygons.iter().map(|p| norm(p)).collect();
    for n in &norms {
        writeln!(output, "vn {} {} {}", n.x, n.y, n.z)?;
    }

    // writeln!(output, "g thing")?;
    // writeln!(output, "usemtl red")?;

    let mut vertex_count = 1;
    // writeln!(output, "s off");
    for (polygon_count, p) in polygons.iter().enumerate() {
        write!(output, "f")?;
        for _ in &p.points {
            write!(
                output,
                " {}/{}/{}",
                vertex_count,
                vertex_count,
                polygon_count + 1
            )?;
            vertex_count += 1;
        }
        writeln!(output)?;
    }
    println!(
        "Wrote {} vertices, {} norms, and {} faces in {} ms.",
        vertex_count - 1,
        norms.len(),
        polygons.len(),
        start.elapsed().as_millis()
    );
    Ok(())
}
