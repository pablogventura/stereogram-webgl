const MAX_TRIANGLES: usize = 2_000_000;

#[derive(Clone)]
pub struct Mesh {
    pub positions: Vec<f32>,
}

impl Mesh {
    pub fn triangle_count(&self) -> u32 {
        (self.positions.len() / 9) as u32
    }

    pub fn from_obj(text: &str) -> Result<Self, String> {
        let mut vertices: Vec<[f32; 3]> = Vec::new();
        let mut positions = Vec::new();

        for (line_index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let command = parts.next().unwrap_or("");
            if command == "v" {
                let coords: Vec<f32> = parts.filter_map(|item| item.parse().ok()).collect();
                if coords.len() < 3 {
                    return Err(format!(
                        "El OBJ tiene un vértice inválido en la línea {}.",
                        line_index + 1
                    ));
                }
                vertices.push([coords[0], coords[1], coords[2]]);
            } else if command == "f" {
                let corners: Vec<&str> = parts.collect();
                if corners.len() < 3 {
                    return Err(format!(
                        "El OBJ tiene una cara inválida en la línea {}.",
                        line_index + 1
                    ));
                }
                for corner in 2..corners.len() {
                    let indices = [
                        face_index(corners[0])?,
                        face_index(corners[corner - 1])?,
                        face_index(corners[corner])?,
                    ];
                    for index in indices {
                        let vertex = vertices.get(index).ok_or_else(|| {
                            format!(
                                "El OBJ referencia un vértice inexistente en la línea {}.",
                                line_index + 1
                            )
                        })?;
                        positions.extend_from_slice(vertex);
                    }
                }
            }
        }

        if positions.is_empty() {
            return Err("El OBJ no tiene triángulos.".to_string());
        }
        Ok(Self { positions })
    }

    pub fn from_stl(bytes: &[u8]) -> Result<Self, String> {
        let positions = if is_binary_stl(bytes) {
            parse_binary_stl(bytes)?
        } else {
            parse_ascii_stl(bytes)?
        };
        if positions.is_empty() || positions.len() % 9 != 0 {
            return Err("El archivo STL no tiene triángulos.".to_string());
        }
        Ok(Self {
            positions: normalize(&positions)?,
        })
    }
}

fn face_index(token: &str) -> Result<usize, String> {
    let index_text = token.split('/').next().unwrap_or("");
    let index: i32 = index_text
        .parse()
        .map_err(|_| format!("Índice de cara inválido: {token}"))?;
    if index < 1 {
        return Err(format!("Índice de cara inválido: {token}"));
    }
    Ok((index - 1) as usize)
}

fn is_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let triangle_count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let expected = 84 + triangle_count * 50;
    triangle_count > 0 && bytes.len() == expected
}

fn parse_binary_stl(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let triangle_count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    if triangle_count > MAX_TRIANGLES {
        return Err("El archivo STL tiene demasiados triángulos.".to_string());
    }
    let mut positions = Vec::with_capacity(triangle_count * 9);
    let mut offset = 84;
    for _ in 0..triangle_count {
        offset += 12;
        for _ in 0..3 {
            for axis in 0..3 {
                let value = f32::from_le_bytes(
                    bytes[offset + axis * 4..offset + axis * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                positions.push(value);
            }
            offset += 12;
        }
        offset += 2;
    }
    Ok(positions)
}

fn parse_ascii_stl(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut positions = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if !line.to_ascii_lowercase().starts_with("vertex") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let _ = parts.next();
        let coords: Vec<f32> = parts.filter_map(|item| item.parse().ok()).collect();
        if coords.len() < 3 || coords.iter().any(|value| !value.is_finite()) {
            return Err("El archivo STL tiene un vértice inválido.".to_string());
        }
        positions.extend_from_slice(&coords[..3]);
        if positions.len() / 9 > MAX_TRIANGLES {
            return Err("El archivo STL tiene demasiados triángulos.".to_string());
        }
    }
    Ok(positions)
}

fn normalize(positions: &[f32]) -> Result<Vec<f32>, String> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in positions.chunks_exact(3) {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    let size = (0..3).map(|axis| max[axis] - min[axis]).fold(0.0, f32::max);
    if !size.is_finite() {
        return Err("El archivo STL tiene un vértice inválido.".to_string());
    }
    let scale = if size > 0.0 { 1.0 / size } else { 1.0 };
    let center = [0, 1, 2].map(|axis| (min[axis] + max[axis]) * 0.5);
    Ok(positions
        .chunks_exact(3)
        .flat_map(|vertex| (0..3).map(move |axis| (vertex[axis] - center[axis]) * scale))
        .collect())
}
