use cgmath::prelude::*;
use cgmath::{Matrix4, Point3, Vector3};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Result, Write};
use std::path::PathBuf;
use std::time::Instant;

pub struct Polygon {
    pub points: Vec<Point3<f32>>,
    pub color: LdrawColor,
}

#[derive(Clone)]
pub enum LdrawColor {
    Main,
    Complement,
    RGBA(f32, f32, f32, f32),
}

impl LdrawColor {
    fn from_str(s: &str) -> Self {
        match s {
            "16" => Self::Main,
            "24" => Self::Complement,
            "0" => Self::RGBA(0.105882, 0.164706, 0.203922, 1.000000),
            "1" => Self::RGBA(0.117647, 0.352941, 0.658824, 1.000000),
            "2" => Self::RGBA(0.000000, 0.521569, 0.168627, 1.000000),
            "3" => Self::RGBA(0.023529, 0.615686, 0.623529, 1.000000),
            "4" => Self::RGBA(0.705882, 0.000000, 0.000000, 1.000000),
            "5" => Self::RGBA(0.827451, 0.207843, 0.615686, 1.000000),
            "6" => Self::RGBA(0.329412, 0.200000, 0.141176, 1.000000),
            "7" => Self::RGBA(0.541176, 0.572549, 0.552941, 1.000000),
            "8" => Self::RGBA(0.329412, 0.349020, 0.333333, 1.000000),
            "9" => Self::RGBA(0.592157, 0.796078, 0.850980, 1.000000),
            "10" => Self::RGBA(0.345098, 0.670588, 0.254902, 1.000000),
            "11" => Self::RGBA(0.000000, 0.666667, 0.643137, 1.000000),
            "12" => Self::RGBA(0.941176, 0.427451, 0.380392, 1.000000),
            "13" => Self::RGBA(0.964706, 0.662745, 0.733333, 1.000000),
            "14" => Self::RGBA(0.980392, 0.784314, 0.039216, 1.000000),
            "15" => Self::RGBA(0.956863, 0.956863, 0.956863, 1.000000),
            "17" => Self::RGBA(0.678431, 0.850980, 0.658824, 1.000000),
            "18" => Self::RGBA(1.000000, 0.839216, 0.498039, 1.000000),
            "19" => Self::RGBA(0.690196, 0.627451, 0.435294, 1.000000),
            "20" => Self::RGBA(0.686275, 0.745098, 0.839216, 1.000000),
            "22" => Self::RGBA(0.403922, 0.121569, 0.505882, 1.000000),
            "23" => Self::RGBA(0.054902, 0.243137, 0.603922, 1.000000),
            "25" => Self::RGBA(0.839216, 0.474510, 0.137255, 1.000000),
            "26" => Self::RGBA(0.564706, 0.121569, 0.462745, 1.000000),
            "27" => Self::RGBA(0.647059, 0.792157, 0.094118, 1.000000),
            "28" => Self::RGBA(0.537255, 0.490196, 0.384314, 1.000000),
            "29" => Self::RGBA(1.000000, 0.619608, 0.803922, 1.000000),
            "30" => Self::RGBA(0.627451, 0.431373, 0.725490, 1.000000),
            "31" => Self::RGBA(0.803922, 0.643137, 0.870588, 1.000000),
            "68" => Self::RGBA(0.992157, 0.764706, 0.513726, 1.000000),
            "69" => Self::RGBA(0.541176, 0.070588, 0.658824, 1.000000),
            "70" => Self::RGBA(0.372549, 0.192157, 0.035294, 1.000000),
            "71" => Self::RGBA(0.588235, 0.588235, 0.588235, 1.000000),
            "72" => Self::RGBA(0.392157, 0.392157, 0.392157, 1.000000),
            "73" => Self::RGBA(0.450980, 0.588235, 0.784314, 1.000000),
            "74" => Self::RGBA(0.498039, 0.768627, 0.458824, 1.000000),
            "77" => Self::RGBA(0.996078, 0.800000, 0.811765, 1.000000),
            "78" => Self::RGBA(1.000000, 0.788235, 0.584314, 1.000000),
            "84" => Self::RGBA(0.666667, 0.490196, 0.333333, 1.000000),
            "85" => Self::RGBA(0.266667, 0.101961, 0.568627, 1.000000),
            "86" => Self::RGBA(0.678431, 0.380392, 0.250980, 1.000000),
            "89" => Self::RGBA(0.109804, 0.345098, 0.654902, 1.000000),
            "92" => Self::RGBA(0.733333, 0.501961, 0.352941, 1.000000),
            "100" => Self::RGBA(0.976471, 0.717647, 0.647059, 1.000000),
            "110" => Self::RGBA(0.149020, 0.274510, 0.603922, 1.000000),
            "112" => Self::RGBA(0.282353, 0.380392, 0.674510, 1.000000),
            "115" => Self::RGBA(0.717647, 0.831373, 0.145098, 1.000000),
            "118" => Self::RGBA(0.611765, 0.839216, 0.800000, 1.000000),
            "120" => Self::RGBA(0.870588, 0.917647, 0.572549, 1.000000),
            "125" => Self::RGBA(0.976471, 0.654902, 0.466667, 1.000000),
            "128" => Self::RGBA(0.678431, 0.380392, 0.250980, 1.000000),
            "151" => Self::RGBA(0.784314, 0.784314, 0.784314, 1.000000),
            "191" => Self::RGBA(0.988235, 0.674510, 0.000000, 1.000000),
            "212" => Self::RGBA(0.615686, 0.764706, 0.968627, 1.000000),
            "216" => Self::RGBA(0.529412, 0.168627, 0.090196, 1.000000),
            "218" => Self::RGBA(0.556863, 0.333333, 0.592157, 1.000000),
            "219" => Self::RGBA(0.337255, 0.305882, 0.615686, 1.000000),
            "226" => Self::RGBA(1.000000, 0.925490, 0.423529, 1.000000),
            "232" => Self::RGBA(0.466667, 0.788235, 0.847059, 1.000000),
            "272" => Self::RGBA(0.098039, 0.196078, 0.352941, 1.000000),
            "288" => Self::RGBA(0.000000, 0.270588, 0.101961, 1.000000),
            "295" => Self::RGBA(1.000000, 0.580392, 0.760784, 1.000000),
            "308" => Self::RGBA(0.207843, 0.129412, 0.000000, 1.000000),
            "313" => Self::RGBA(0.670588, 0.850980, 1.000000, 1.000000),
            "320" => Self::RGBA(0.447059, 0.000000, 0.070588, 1.000000),
            "321" => Self::RGBA(0.274510, 0.607843, 0.764706, 1.000000),
            "322" => Self::RGBA(0.407843, 0.764706, 0.886275, 1.000000),
            "323" => Self::RGBA(0.827451, 0.949020, 0.917647, 1.000000),
            "326" => Self::RGBA(0.886275, 0.976471, 0.603922, 1.000000),
            "330" => Self::RGBA(0.466667, 0.466667, 0.305882, 1.000000),
            "335" => Self::RGBA(0.533333, 0.376471, 0.368627, 1.000000),
            "351" => Self::RGBA(0.968627, 0.521569, 0.694118, 1.000000),
            "353" => Self::RGBA(1.000000, 0.427451, 0.466667, 1.000000),
            "366" => Self::RGBA(0.847059, 0.427451, 0.172549, 1.000000),
            "373" => Self::RGBA(0.458824, 0.396078, 0.490196, 1.000000),
            "378" => Self::RGBA(0.439216, 0.556863, 0.486275, 1.000000),
            "379" => Self::RGBA(0.439216, 0.505882, 0.603922, 1.000000),
            "450" => Self::RGBA(0.823529, 0.466667, 0.266667, 1.000000),
            "462" => Self::RGBA(0.960784, 0.525490, 0.141176, 1.000000),
            "484" => Self::RGBA(0.568627, 0.313726, 0.109804, 1.000000),
            "503" => Self::RGBA(0.737255, 0.705882, 0.647059, 1.000000),
            "507" => Self::RGBA(0.980392, 0.611765, 0.109804, 1.000000),
            "508" => Self::RGBA(1.000000, 0.501961, 0.078431, 1.000000),
            "509" => Self::RGBA(0.811765, 0.541176, 0.278431, 1.000000),
            "510" => Self::RGBA(0.470588, 0.988235, 0.470588, 1.000000),
            "33" => Self::RGBA(0.000000, 0.125490, 0.627451, 0.501961),
            "34" => Self::RGBA(0.137255, 0.470588, 0.254902, 0.501961),
            "35" => Self::RGBA(0.337255, 0.901961, 0.274510, 0.501961),
            "36" => Self::RGBA(0.788235, 0.101961, 0.035294, 0.501961),
            "37" => Self::RGBA(0.874510, 0.400000, 0.584314, 0.501961),
            "38" => Self::RGBA(1.000000, 0.501961, 0.050980, 0.501961),
            "39" => Self::RGBA(0.756863, 0.874510, 0.941176, 0.501961),
            "40" => Self::RGBA(0.388235, 0.372549, 0.321569, 0.501961),
            "41" => Self::RGBA(0.333333, 0.603922, 0.717647, 0.501961),
            "42" => Self::RGBA(0.752941, 1.000000, 0.000000, 0.501961),
            "43" => Self::RGBA(0.682353, 0.913725, 0.937255, 0.501961),
            "44" => Self::RGBA(0.588235, 0.439216, 0.623529, 0.501961),
            "45" => Self::RGBA(0.988235, 0.592157, 0.674510, 0.501961),
            "46" => Self::RGBA(0.960784, 0.803922, 0.184314, 0.501961),
            "47" => Self::RGBA(0.988235, 0.988235, 0.988235, 0.501961),
            "52" => Self::RGBA(0.647059, 0.647059, 0.796078, 0.501961),
            "54" => Self::RGBA(0.854902, 0.690196, 0.000000, 0.501961),
            "57" => Self::RGBA(0.941176, 0.560784, 0.109804, 0.501961),
            "231" => Self::RGBA(0.988235, 0.717647, 0.427451, 0.501961),
            "234" => Self::RGBA(0.984314, 0.909804, 0.564706, 0.501961),
            "284" => Self::RGBA(0.760784, 0.505882, 0.647059, 0.501961),
            "285" => Self::RGBA(0.490196, 0.760784, 0.568627, 0.501961),
            "293" => Self::RGBA(0.419608, 0.670588, 0.894118, 0.501961),
            "334" => Self::RGBA(0.874510, 0.756863, 0.462745, 1.000000),
            "383" => Self::RGBA(0.807843, 0.807843, 0.807843, 1.000000),
            "60" => Self::RGBA(0.392157, 0.352941, 0.298039, 1.000000),
            "64" => Self::RGBA(0.105882, 0.164706, 0.203922, 1.000000),
            "61" => Self::RGBA(0.423529, 0.588235, 0.749020, 1.000000),
            "62" => Self::RGBA(0.235294, 0.701961, 0.443137, 1.000000),
            "63" => Self::RGBA(0.666667, 0.301961, 0.556863, 1.000000),
            "183" => Self::RGBA(0.964706, 0.949020, 0.874510, 1.000000),
            "150" => Self::RGBA(0.596078, 0.607843, 0.600000, 1.000000),
            "135" => Self::RGBA(0.627451, 0.627451, 0.627451, 1.000000),
            "179" => Self::RGBA(0.537255, 0.529412, 0.533333, 1.000000),
            "148" => Self::RGBA(0.282353, 0.301961, 0.282353, 1.000000),
            "137" => Self::RGBA(0.356863, 0.458824, 0.564706, 1.000000),
            "142" => Self::RGBA(0.870588, 0.674510, 0.400000, 1.000000),
            "297" => Self::RGBA(0.666667, 0.498039, 0.180392, 1.000000),
            "178" => Self::RGBA(0.513726, 0.447059, 0.309804, 1.000000),
            "134" => Self::RGBA(0.462745, 0.301961, 0.231373, 1.000000),
            "189" => Self::RGBA(0.674510, 0.509804, 0.278431, 1.000000),
            "80" => Self::RGBA(0.462745, 0.462745, 0.462745, 1.000000),
            "81" => Self::RGBA(0.415686, 0.474510, 0.266667, 1.000000),
            "82" => Self::RGBA(0.858824, 0.674510, 0.203922, 1.000000),
            "83" => Self::RGBA(0.039216, 0.074510, 0.152941, 1.000000),
            "87" => Self::RGBA(0.427451, 0.431373, 0.360784, 1.000000),
            "300" => Self::RGBA(0.760784, 0.498039, 0.325490, 1.000000),
            "184" => Self::RGBA(0.839216, 0.000000, 0.149020, 1.000000),
            "186" => Self::RGBA(0.000000, 0.556863, 0.235294, 1.000000),
            "79" => Self::RGBA(0.933333, 0.933333, 0.933333, 0.941176),
            "21" => Self::RGBA(0.878431, 1.000000, 0.690196, 0.941176),
            "294" => Self::RGBA(0.741176, 0.776471, 0.678431, 0.941176),
            "329" => Self::RGBA(0.960784, 0.952941, 0.843137, 0.941176),
            "114" => Self::RGBA(0.874510, 0.400000, 0.584314, 0.501961),
            "117" => Self::RGBA(0.933333, 0.933333, 0.933333, 0.501961),
            "129" => Self::RGBA(0.392157, 0.000000, 0.380392, 0.501961),
            "302" => Self::RGBA(0.682353, 0.913725, 0.937255, 0.501961),
            "339" => Self::RGBA(0.752941, 1.000000, 0.000000, 0.501961),
            "132" => Self::RGBA(0.000000, 0.000000, 0.000000, 1.000000),
            "133" => Self::RGBA(0.000000, 0.000000, 0.000000, 1.000000),
            "75" => Self::RGBA(0.000000, 0.000000, 0.000000, 1.000000),
            "76" => Self::RGBA(0.388235, 0.372549, 0.380392, 1.000000),
            "65" => Self::RGBA(0.980392, 0.784314, 0.039216, 1.000000),
            "66" => Self::RGBA(0.960784, 0.803922, 0.184314, 0.501961),
            "67" => Self::RGBA(0.988235, 0.988235, 0.988235, 0.501961),
            "256" => Self::RGBA(0.105882, 0.164706, 0.203922, 1.000000),
            "273" => Self::RGBA(0.117647, 0.352941, 0.658824, 1.000000),
            "324" => Self::RGBA(0.705882, 0.000000, 0.000000, 1.000000),
            "350" => Self::RGBA(0.839216, 0.474510, 0.137255, 1.000000),
            "375" => Self::RGBA(0.541176, 0.572549, 0.552941, 1.000000),
            "406" => Self::RGBA(0.098039, 0.196078, 0.352941, 1.000000),
            "449" => Self::RGBA(0.403922, 0.121569, 0.505882, 1.000000),
            "490" => Self::RGBA(0.647059, 0.792157, 0.094118, 1.000000),
            "496" => Self::RGBA(0.588235, 0.588235, 0.588235, 1.000000),
            "504" => Self::RGBA(0.537255, 0.529412, 0.533333, 1.000000),
            "511" => Self::RGBA(0.956863, 0.956863, 0.956863, 1.000000),
            "10002" => Self::RGBA(0.345098, 0.670588, 0.254902, 1.000000),
            "10026" => Self::RGBA(0.564706, 0.121569, 0.462745, 1.000000),
            "10030" => Self::RGBA(0.627451, 0.431373, 0.725490, 1.000000),
            "10031" => Self::RGBA(0.803922, 0.643137, 0.870588, 1.000000),
            "10070" => Self::RGBA(0.372549, 0.192157, 0.035294, 1.000000),
            "10226" => Self::RGBA(1.000000, 0.925490, 0.423529, 1.000000),
            "10308" => Self::RGBA(0.207843, 0.129412, 0.000000, 1.000000),
            "10320" => Self::RGBA(0.447059, 0.000000, 0.070588, 1.000000),
            "10321" => Self::RGBA(0.274510, 0.607843, 0.764706, 1.000000),
            "10322" => Self::RGBA(0.407843, 0.764706, 0.886275, 1.000000),
            "10323" => Self::RGBA(0.827451, 0.949020, 0.917647, 1.000000),
            "10484" => Self::RGBA(0.568627, 0.313726, 0.109804, 1.000000),
            "32" => Self::RGBA(0.000000, 0.000000, 0.000000, 0.823529),
            "493" => Self::RGBA(0.396078, 0.403922, 0.380392, 1.000000),
            "494" => Self::RGBA(0.815686, 0.815686, 0.815686, 1.000000),
            "495" => Self::RGBA(0.682353, 0.478431, 0.349020, 1.000000),
            "10047" => Self::RGBA(1.000000, 1.000000, 1.000000, 0.062745),
            _ => Self::Main,
        }
    }
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
    let mut polygons = Vec::new();
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
        PathBuf::new().join(".").join(&filename),
    ];
    let mut my_path = PathBuf::new();
    for path in paths {
        if path.exists() {
            my_path = path.clone();
            break;
        }
    }

    if !my_path.exists() {
        println!("WARNING: Couldn't find part: {}. Skipping...", filename);
        return polygons;
    }
    let input = File::open(my_path).unwrap();

    let input = BufReader::new(input);

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
                        let color = LdrawColor::from_str(maybe_color);
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
                            match &new_polygon.color {
                                LdrawColor::Main => {
                                    new_polygon.color = color.clone();
                                }
                                _ => {}
                            };
                            polygons.push(new_polygon);
                        }
                    }
                    "2" => {
                        // TODO line
                    }
                    "3" => {
                        if data.len() == 9 {
                            let mut polygon = Polygon {
                                points: Vec::new(),
                                color: LdrawColor::from_str(maybe_color),
                            };
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
                            let mut polygon = Polygon {
                                points: Vec::new(),
                                color: LdrawColor::from_str(maybe_color),
                            };
                            let mut polygon2 = Polygon {
                                points: Vec::new(),
                                color: LdrawColor::from_str(maybe_color),
                            };
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
